#!/usr/bin/env python3

import os
import json
import requests

SNIPPET_ID = 3607411


def publish_data(filename, contents):
    URL = "https://gitlab.com/api/v4/snippets/" + str(SNIPPET_ID)

    headers = {"Content-Type": "application/json", "PRIVATE-TOKEN": os.environ.get("GITLAB_API_TOKEN")}

    payload = {"files": [{"action": "update", "content": contents, "file_path": filename}]}
    r = requests.put(URL, data=json.dumps(payload), headers=headers)

    if 400 <= r.status_code < 500:
        payload = {"files": [{"action": "create", "content": contents, "file_path": filename}]}
        r = requests.put(URL, data=json.dumps(payload), headers=headers)

    r.raise_for_status()

def main():
    contents = "this,is,a,test"
    publish_data("staging_guests.csv", contents=contents)


if __name__ == "__main__":
    main()
