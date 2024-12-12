import subprocess
import urllib.request
import json
import pathlib

def get_root() -> str:
    output = subprocess.run(["git", "rev-parse", "--show-toplevel"], check=True, text=True, stdout=subprocess.PIPE)
    return output.stdout.strip()

def get_subnets_data() -> dict:
    req = urllib.request.Request(url='https://ic-api.internetcomputer.org/api/v3/subnets?format=json', headers={
        "user-agent": "python"
    })

    subnets = {}
    with urllib.request.urlopen(req, timeout=30) as response:
        subnets_data = json.loads(response.read().decode())["subnets"]
        for subnet in subnets_data:
            subnets[subnet["subnet_id"]] = {
                "topic_id": 0,
                "slug": ""
            }

    return subnets

def fetch_existing_data(path) -> dict:
    with open(path, "r") as f:
        return json.load(f)

if __name__ == "__main__":
    root = pathlib.Path(get_root())
    all_subnets = get_subnets_data()
    subnet_topic_file = root / "rs" / "cli" / "src" / "assets" / "subnet_topic_map.json"
    existing_subnets = fetch_existing_data(subnet_topic_file)
    for subnet in all_subnets:
        if subnet in existing_subnets:
            print(f"Subnet '{subnet}' already present")
            continue

        existing_subnets[subnet] = all_subnets[subnet]

    with open(subnet_topic_file, "w+") as f:
        json.dump(existing_subnets, f, indent=4)
