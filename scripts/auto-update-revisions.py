import json
import subprocess
from pathlib import Path
import re

SKIP = [
	"Cargo.lock",
	"Cargo.Bazel.lock",
	"target/.*",
	".*\.png$",
	".*\.jpeg$",
	".*\.md$",
	".*\.ico$",
	"\.git/.*"
]

SKIP_REGEX = [re.compile(f".*/{pattern}") for pattern in SKIP]

def get_toplevel() -> str:
	return subprocess.run([
		"git",
		"rev-parse",
		"--show-toplevel"
	], capture_output=True, text=True, check=True).stdout.strip()

def get_latest_commit(repo_url: str, ref: str) -> str:
	return subprocess.run([
		"git",
		"ls-remote",
		repo_url,
		ref
	], capture_output=True, text=True, check=True).stdout.strip().split()[0] 

def get_files(top_level: str):
	path = Path(top_level)
	for file_path in path.rglob('**/*'):
		if file_path.is_file() and not any([bool(regex.fullmatch(str(file_path))) for regex in SKIP_REGEX]):
			yield file_path

def update_files(top_level: str, to_update: list[(str, str)]):
	for file_path in get_files(top_level):
		try:
			with open(file_path, 'r') as file:
				content = file.read()

			for (from_commit, to_commit) in to_update:
				content = content.replace(from_commit, to_commit)

			with open(file_path, 'w') as file:
				file.write(content)
		except Exception as e:
			print(f"Error on path '{file_path}': {e}")
	pass

def main():
	top_level = get_toplevel()
	deps = json.load(open(f"{top_level}/ic-revisions.json"))
	to_update = []

	for key in deps:
		dep = deps[key]
		print(f"Updating {key} from {dep['commit']} and ref {dep['ref']}")
		new_commit = get_latest_commit(key, dep["ref"])
		if dep["commit"] == new_commit:
			print("Nothing new... skipping")
			continue
		
		print(f"Will update {key}: {dep['commit']} > {new_commit}")
		to_update.append((dep['commit'], new_commit))
		dep['commit'] = new_commit
	
	print("Updating files")
	update_files(top_level, to_update)

	print("Updating revisions file")
	json.dump(deps, open(f"{top_level}/ic-revisions.json", mode="w"), indent=4)
	print("Done")

if __name__ == "__main__":
	main()