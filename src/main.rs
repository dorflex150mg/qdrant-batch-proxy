//! # Auto-Batching Proxy Service
//!
//! This service wraps a Hugging Face `text-embeddings-inference` server with an Actix Web
//! API that looks like a single-request interface, but internally batches requests for
//! efficiency.
//!
//! ## Overview
//!
//! - **Incoming requests** are posted to `/embed` with a JSON body containing an `"inputs"` field.
//! - Each request is turned into a [`Job`] and queued in an `mpsc` channel.
//! - A background [`handle_batch`] task collects jobs into batches until either:
//!   - [`MAX_BATCH_SIZE`] is reached, or
//!   - [`MAX_WAIT_TIME_MILLIS`] milliseconds have passed since the first job in the batch.
//! - The batch is sent as a single request to the upstream Hugging Face inference service at
//!   [`TARGET_SERVICE_URL`].
//! - The upstream response is split back into individual responses in the same order,
//!   and sent to the original requestors via `oneshot` channels.
//!
//! ## API
//!
//! **Request**:
//! ```json
//! { "inputs": "some text" }
//! ```
//!
//! **Response** (example):
//! ```json
//! {
//!   "embedding": [0.0123, -0.0456, 0.0789]
//! }
//! ```
//!
//! The upstream server may return either:
//! - An array of float arrays (batch mode).
//! - An object with a `"data"` key containing the array of float arrays.
//!
//! This service handles both cases.
//!
//! ## Constants
//!
//! - [`MAX_BATCH_SIZE`]: Maximum number of jobs in one batch.
//! - [`MAX_WAIT_TIME_MILLIS`]: Maximum time to wait for batch filling.
//! - [`TARGET_SERVICE_URL`]: URL of the upstream embedding service.
//!
//! ## Usage
//!
//! ```bash
//! curl 127.0.0.1:3000/embed \
//!   -H "Content-Type: application/json" \
//!   -d '{"inputs":"Hello world"}'
//! ```
//!
//! ## Logging
//!
//! This service uses [`tracing`] for debug logs. Logs include batch size, timeouts, and
//! errors from the upstream service.
//!
//! ## Types
//!
//! - [`EmbedRequest`]: Incoming request payload.
//! - [`EmbedResponse`]: Outgoing response payload.
//! - [`Job`]: Internal struct containing the text to embed and a channel to send the embedding back.
//!
//!

use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

use actix_web::{web, App, HttpServer, Responder};
use serde::{Deserialize, Serialize};

const MAX_BATCH_SIZE: usize = 32; // maximum the server accepts
const MAX_WAIT_TIME_MILLIS: u64= 10_000; //10 sec
const TARGET_SERVICE_URL: &str = "http://127.0.0.1:8080/embed";


#[derive(Debug, Deserialize)]
struct EmbedRequest {
    inputs: String,
}

#[derive(Debug, Serialize)]
struct EmbedResponse {
    embedding: serde_json::Value,
}


#[derive(Debug)]
struct Job {
    input: String,
    inner_sender: oneshot::Sender<serde_json::Value>,
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();


    let (job_sender, job_receiver) = mpsc::channel::<Job>(1024);
    tokio::spawn(handle_batch(job_receiver));

    HttpServer::new(move || {
        tracing::debug!("Server started.");
        App::new()
            .app_data(web::Data::new(job_sender.clone()))
            .route("/embed", web::post().to(handle_query))
        })
        .bind(("0.0.0.0", 3000))?
        .run()
        .await
}

async fn handle_query(sender: web::Data<mpsc::Sender<Job>>, payload: web::Json<EmbedRequest>) -> impl Responder {
    let (inner_sender, inner_receiver) = oneshot::channel();
    let job = Job {
        input: payload.inputs.clone(),
        inner_sender
    };
    if !sender.send(job).await.is_ok() {
        return web::Json(EmbedResponse { embedding: serde_json::json!("Failed to batch.")});
    }
    match inner_receiver.await {
        Ok(response) => {
            web::Json(EmbedResponse {embedding: response})
        }
        Err(_) => web::Json(EmbedResponse { embedding: serde_json::json!("Batching Failed. Try again.")}),
    }
}

async fn handle_batch(mut job_receiver: mpsc::Receiver<Job>) {
    let client = reqwest::Client::new(); // better create the client before the batch
                                                 // starts.
    loop {
        let mut batch: Vec<Job> = vec![];
        let alarm_clock = tokio::time::sleep(Duration::from_millis(MAX_WAIT_TIME_MILLIS));
        tokio::pin!(alarm_clock);
        while batch.len() < MAX_BATCH_SIZE {
            tokio::select! {
                received = job_receiver.recv() => {
                    match received {
                        Some(job) => {
                            batch.push(job);
                            if batch.len() > MAX_BATCH_SIZE {
                            tracing::debug!("Batch Maxed out");
                                break;
                            }
                        },
                        None => break,
                    }
                }
                _ =  &mut alarm_clock => {
                    tracing::debug!("Batch Timed out");
                    break; 
                }
            }
        }
        let copies_to_send: Vec<String> = batch.iter().map(|x| x.input.clone()).collect();
        match client.post(TARGET_SERVICE_URL) 
            .json(&serde_json::json!({"inputs": copies_to_send}))
            .send()
            .await {
                Ok(batch_response) => {
                    let json: Result<serde_json::Value, reqwest::Error> = batch_response.json().await; 
                    match json {
                        Ok(val) => {
                            let results = if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
                                data.clone()
                            } else if let Some(arr) = val.as_array() {
                                arr.clone()
                            } else {
                                vec![]
                            };
                            for (job, result) in batch.into_iter().zip(results.into_iter()) {
                                let _ = job.inner_sender.send(result);
                            }
                        },
                        Err(_) => {
                            batch
                                .into_iter()
                                .for_each(|job| {
                                    _ = job.inner_sender.send(serde_json::json!({"error": "upstream failed"}));
                                });
                        }
                    }
                    },
                Err(_) => {

                    batch
                        .into_iter()
                        .for_each(|job| {
                            _ = job.inner_sender.send(serde_json::json!({"error": "upstream failed"}));
                        });

                }
        }
    }
}
