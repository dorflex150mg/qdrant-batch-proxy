function ProgressBar {
# Process data
    let _progress=(${1}*100/${2}*100)/100
    let _done=(${_progress}*4)/10
    let _left=40-$_done
# Build progressbar string lengths
    _fill=$(printf "%${_done}s")
    _empty=$(printf "%${_left}s")

# 1.2 Build progressbar strings and print the ProgressBar line
# 1.2.1 Output example:                           
# 1.2.1.1 Progress : [########################################] 100%
printf "\rProgress : [${_fill// /#}${_empty// /-}] ${_progress}%%"

}

rm partials_bulk.csv
rm absolutes_bulk.csv
count=0
start_time=$(date +%s%N)
for i in $(seq 1 1000); do
    ProgressBar $i 1000
    bash make_daemon_request.sh $count &
done;
end_time=$(date +%s%N)
elapsed=$(($end_time - $start_time))
echo -e "\nElapsed time:\t\t$elapsed"
echo $elapsed >> finals_bulk.csv


