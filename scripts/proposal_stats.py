#!/usr/bin/env python3
import datetime
import sys

import requests

if len(sys.argv[1:]) != 2:
    print("Usage: ./proposal_stats.py <year> <month>")
    sys.exit(1)

year = int(sys.argv[1])
month = int(sys.argv[2])
start = datetime.datetime(year, month, 1, 0, 0, 0)

neurons = {
    40: "Sasa",
    47: "Luis",
    39: "Luka",
}

page = 0
limit = 100
proposals = []
finished = False
while not finished:
    r = requests.get(
        "https://ic-api.internetcomputer.org/api/v3/proposals?offset={}&limit={}".format(page * limit, limit),
        timeout=60,
    )
    for e in r.json()["data"]:
        date = datetime.datetime.fromtimestamp(e["proposal_timestamp_seconds"])
        if date.month == month and date.year == year and int(e["proposer"]) in neurons.keys():
            proposals.append(e)
        if date < start:
            finished = True
            break
    page += 1

error_count = 0
for p in proposals:
    if p["status"] != "EXECUTED":
        error_count += 1
        print(
            "{} - {} by {} [{}]".format(
                datetime.datetime.fromtimestamp(p["proposal_timestamp_seconds"]),
                p["proposal_id"],
                neurons[int(p["proposer"])],
                p["status"],
            )
        )

print("{} out of {} proposals with non-executed status".format(error_count, len(proposals)))
