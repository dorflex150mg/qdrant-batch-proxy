inner_start_time=$(date +%s%N)
curl -sS 127.0.0.1:3000/embed -X POST -d '{"inputs": "What is Vector Search? $1"}' -H 'Content-Type: application/json' > /dev/null
inner_end_time=$(date +%s%N)
echo $(($inner_end_time - $inner_start_time)) >> partials_bulk.csv
echo $inner_end_time >> absolutes_bulk.csv



