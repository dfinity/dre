#!/usr/bin/env python3
import argparse
import json
import os
import pathlib
import re
import subprocess
import tempfile
import webbrowser

parser = argparse.ArgumentParser(description="Generate release notes")
parser.add_argument("first_commit", type=str, help="first commit")
parser.add_argument("last_commit", type=str, help="last commit")
parser.add_argument("--max-commits", dest="max_commits", default=1000, help="maximum number of commits to fetch")
parser.add_argument("--branch", dest="branch", help="branch to fetch commits from")
parser.add_argument(
    "--html",
    type=str,
    dest="html_path",
    default="$HOME/Downloads/release-notes.html",
    help="path to where the output should be generated",
)
parser.add_argument("rc_name", type=str, help="name of the release i.e. 'rc--2023-01-12_18-31'")
args = parser.parse_args()

max_commits = os.environ.get("MAX_COMMITS", args.max_commits)
branch = os.environ.get("BRANCH") or args.branch or args.rc_name

def get_merge_commit(repo_dir, commit_hash, branch):
    relevant_commits = list(enumerate(get_ancestry_path(repo_dir, commit_hash, branch)))
    relevant_commits += list(enumerate(get_first_parent(repo_dir, commit_hash, branch)))
    relevant_commits = sorted(relevant_commits, key=lambda index_commit: index_commit[1])
    checked_commits = set()
    commits = []
    for index, commit in relevant_commits:
        if commit not in checked_commits:
            checked_commits.add(commit)
            commits.append((index, commit))

    relevant_commits = sorted(commits, key=lambda index_commit: index_commit[0])

    return relevant_commits[-1][1]


def get_ancestry_path(repo_dir, commit_hash, branch):
    return (
        subprocess.check_output(
            [
                "git",
                "--git-dir",
                "{}/.git".format(repo_dir),
                "rev-list",
                "{}..{}".format(commit_hash, branch),
                "--ancestry-path",
            ]
        )
        .decode("utf-8")
        .strip()
        .split("\n")
    )


def get_first_parent(repo_dir, commit_hash, branch):
    return (
        subprocess.check_output(
            [
                "git",
                "--git-dir",
                "{}/.git".format(repo_dir),
                "rev-list",
                "{}..{}".format(commit_hash, branch),
                "--ancestry-path",
            ]
        )
        .decode("utf-8")
        .strip()
        .split("\n")
    )


def get_commits(repo_dir, first_commit, last_commit):
    def get_commits_info(git_commit_format):
        return (
            subprocess.check_output(
                [
                    "git",
                    "--git-dir={}/.git".format(repo_dir),
                    "log",
                    "--format={}".format(git_commit_format),
                    "--no-merges",
                    "{}..{}".format(first_commit, last_commit),
                ]
            )
            .decode("utf-8")
            .strip()
            .split("\n")
        )

    return list(zip(get_commits_info("%h"), get_commits_info("%cD"), get_commits_info("%an"), get_commits_info("%s")))


def main():
    first_commit = args.first_commit
    last_commit = args.last_commit
    html_path = os.path.expandvars(args.html_path)
    rc_name = args.rc_name

    with tempfile.TemporaryDirectory() as temp_ic_repo:
        subprocess.check_call(
            [
                "git",
                "clone",
                "--depth={}".format(max_commits),
                "--filter=blob:none",
                "--no-checkout",
                "--single-branch",
                "--branch={}".format(branch),
                "https://github.com/dfinity/ic.git",
                temp_ic_repo,
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        commits = get_commits(temp_ic_repo, first_commit, last_commit)
        for i in range(len(commits)):
            commits[i] = commits[i] + (str(get_merge_commit(temp_ic_repo, commits[i][0], branch)),)

    if len(commits) == max_commits:
        print("WARNING: max commits limit reached, increase depth")
        exit(1)

    # Current team membership can be found at https://www.notion.so/Teams-864f8176074b4bc7896147f4d1246b54
    teams = json.load(open(pathlib.Path(__file__).parent / "teams.json", encoding="utf8"))

    release_notes = []
    excluded_changes = []
    errors = set()
    replica_teams = [
        "Consensus",
        "Crypto",
        "Orchestrator",
        "Message Routing",
        "Networking",
        "Execution",
        "Node",
        "Runtime",
    ]

    jira_ticket_regex = r" *\b[A-Z]{2,}\d?-\d+\b:?"  # <whitespace?><word boundary><uppercase letters><digit?><hyphen><digits><word boundary><colon?>
    empty_brackets_regex = r" *\[ *\]:?"  # Sometimes Jira tickets are in square brackets

    has_crossed_out_changes = False
    for (_abbrv_commit_hash, _date, author, message, merge_commit) in commits:
        authors_teams = [team for team, members in teams.items() if author in members]
        if len(authors_teams) == 0:
            errors.add("ERROR: author '{}' does not belong in any team".format(author))

        stripped_message = re.sub(jira_ticket_regex, "", message)
        stripped_message = re.sub(empty_brackets_regex, "", stripped_message)
        stripped_message = stripped_message.strip()

        change = '* [<a href="https://github.com/dfinity/ic/commit/{0}">{0}</a>] {1}: {2}<br>'.format(
            merge_commit[0:9], "/".join(authors_teams), stripped_message
        )
        if any([authors_team in replica_teams for authors_team in authors_teams]):
            if any([term in change.lower() for term in ["test", "refactor"]]):
                release_notes.append("<s>{}</s>".format(change))
                has_crossed_out_changes = True
            else:
                release_notes.append(change)
        else:
            excluded_changes.append("<s>{}</s>".format(change))

    if len(errors) > 0:
        print("\n".join(errors))
        exit(1)

    release_notes = sorted(
        release_notes, key=lambda a: a[a.index("]") + 2 :]
    )  # Sort without including abbrv_commit_hash
    if has_crossed_out_changes:
        release_notes.append("* Various tech-debt management: code refactoring, docs, bug fixes, test updates")

    with open(html_path, "w", encoding="utf-8") as output:
        output.write(
            """
        <link rel="preconnect" href="https://fonts.googleapis.com">
        <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
        <link href="https://fonts.googleapis.com/css2?family=Roboto+Mono&display=swap" rel="stylesheet">
        """
        )
        output.write('<div style="font-family: Courier New; font-size: 8pt">')
        output.write(
            '<h1 id="{0}" style="font-size: 14pt; font-family: Roboto">Release Notes for <a style="font-size: 10pt; font-family: Roboto Mono" href="https://github.com/dfinity/ic/tree/{0}">{0}</a> <span style="font-family: Roboto Mono; font-weight: normal; font-size: 10pt">({1})</span></h1>\n'.format(
                rc_name, last_commit
            )
        )
        output.write(
            "\n".join(["<p style='font-size: 8pt; padding: 0; margin: 0'>{}</p>".format(n) for n in release_notes])
        )

        output.write("<h2 style='font-family: Roboto; font-size: 14pt'><u>Excluded changes:</u></h2>\n")
        output.write(
            "\n".join(
                [
                    "<p style='font-size: 8pt; padding: 0; margin: 0'>{}</p>".format(c)
                    for c in sorted(excluded_changes, key=lambda a: a[a.index("]") + 2 :])
                ]
            )
        )

        output.write("</div>")

    filename = "file:///{}".format(html_path)
    webbrowser.open_new_tab(filename)


if __name__ == "__main__":
    main()
