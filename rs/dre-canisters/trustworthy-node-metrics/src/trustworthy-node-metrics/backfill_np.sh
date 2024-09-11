#!/bin/bash

# Define the path to the file containing the list of principals
input_file="/Users/pietro.di.marco/Documents/dfinity/dre-1/rs/dre-canisters/trustworthy-node-metrics/src/trustworthy-node-metrics/node_ids.txt"

# Loop through each line in the input file
while IFS= read -r line; do
  # Extract the principal value from the line (assuming the line itself is the principal)
  principal="$line"
  
  # Execute the dfx canister call with the extracted principal
  time dfx canister call trustworthy-node-metrics \
    --candid '/Users/pietro.di.marco/Documents/dfinity/dre/rs/dre-canisters/trustworthy-node-metrics/src/trustworthy-node-metrics/trustworthy-node-metrics.did' \
    --network ic \
    update_node_provider "(principal \"$principal\")"
    
done < "$input_file"
