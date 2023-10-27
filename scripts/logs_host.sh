#!/bin/bash

# Example usage:
# ./logs_host.sh 2600:3006:1400:1500:6800:e8ff:feca:998c
URL="http://[$1]:19531/entries"
if [[ -t 1 ]]; then
    # If interactive (terminal) then get the last 500 lines and follow
    curl -H 'Range: entries=:-500:1000' "$URL?follow"
else
    # If not interactive (e.g. output goes to a pipe) then get all entries and filter out the junk
    curl "$URL" | grep -vE "(Error dialing dial tcp|Client.Timeout exceeded while awaiting headers|Failed to connect to backoff|hostos_guestos_first_boot_state state.prom|DEBUG:root:|INFO:root:|journalbeat|:1: Document is empty|Monitor GuestOS virtual machine|monitor-guestos.service: Succeeded.|Failed to detach device from /tmp|\(null\)|python3\[|         \^|vsock-agent.service|HSM Agent|chronyd\[|motd-news.service|audit.+permissive=1|Message of the Day)"
fi
