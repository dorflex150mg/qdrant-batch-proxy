 Auto-Batching Proxy Service

 This service wraps a Hugging Face [`text-embeddings-inference`](https://github.com/huggingface/text-embeddings-inference) server with an Actix Web API that looks like a single-request interface, but internally batches requests for efficiency.

 ## Overview

 - Incoming requests are posted to `/embed` with a JSON body containing an `"inputs"` field.
 - Each request is turned into a Job and queued in an `mpsc` channel.
 - A background `handle_batch` task collects jobs into batches until either:
   - `MAX_BATCH_SIZE` is reached, or
   - `MAX_WAIT_TIME_MILLIS` milliseconds have passed since the first job in the batch.
 - The batch is sent as a single request to the upstream Hugging Face inference service at `TARGET_SERVICE_URL`.
 - The upstream response is split back into individual responses in the same order, and sent to the original requestors via `oneshot` channels.

 ## API

 Request:
 ```json
 { "inputs": "some text" }
 ```

 Response (example):
 ```json
 {
   "embedding": [0.0123, -0.0456, 0.0789]
 }
 ```

 ## Constants

 - `MAX_BATCH_SIZE`: Maximum number of jobs in one batch.
 - `MAX_WAIT_TIME_MILLIS`: Maximum time to wait for batch filling.
 - `TARGET_SERVICE_URL`: URL of the upstream embedding service.

 ## Usage

 First, run the Hugging Face inference container:

 ```bash
 docker run --rm -it -p 8080:80 \
     --pull always ghcr.io/huggingface/text-embeddings-inference:cpu-latest \
     --model-id nomic-ai/nomic-embed-text-v1.5
 ```

 Then start this proxy server:

 ```bash
 cargo run
 ```

 Make a request:

 ```bash
 curl 127.0.0.1:3000/embed \
   -H "Content-Type: application/json" \
   -d '{"inputs":"Hello world"}'
 ```

Or make multiple requests:

```bash
bash generate_bulk_requests.sh
```
                              
 ## Logging                   
                              
 This service uses [`tracing`](https://docs.rs/tracing) for debug logs.  
 Logs include:                
 - Batch size                 
 - Timeouts                   
 - Errors from the upstream service
                              
 ## Types                     
                              
 - EmbedRequest: Incoming request payload.
 - EmbedResponse: Outgoing response payload.
 - Job: Internal struct containing the text to embed and a channel to send the embedding back.
                              
 ## How It Works              
                              
 1. Client sends request to `/embed`.
 2. Request is queued in a channel as a `Job`.
 3. Batch worker collects jobs until either max size or timeout.
 4. Worker sends batch to upstream Hugging Face service.
 5. Worker splits response and sends each result back to its original requestor.
                              
 ---                          
