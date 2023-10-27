#!/bin/bash

# Example usage:
# ./logs_guest.sh 2401:3f00:1000:23:5000:7bff:fe3d:b81d
URL="http://[$1]:19531/entries"
if [[ -t 1 ]]; then
    # If interactive (terminal) then get the last 500 lines and follow
    curl -6 -H 'Range: entries=:-500:1000' "$URL?follow"
else
    # If not interactive (e.g. output goes to a pipe) then get all entries and filter out the junk
    curl "$URL" | grep -vE "(ic-onchain-observability-adapter|IC Onchain Observability Canister|journalbeat)"
fi
