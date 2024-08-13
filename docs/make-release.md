
# How to make a new release

Go to the repo root, and check that you don't have any dirty changes and that you are on the main branch.

```
git checkout main
git pull
git status
```

If all looks okay, you can run the following convenience script to change the versions in the repo code, and checkout out the new branch and push it to github.

Example:

```
❯ ./bin/mk-release.py 0.5.0
INFO: Updating version from 0.4.3 to 0.5.0
Already up to date.
INFO: Patching file pyproject.toml
INFO:   ---
INFO:   +++
INFO:   @@ -1,6 +1,6 @@
INFO:    [tool.poetry]
INFO:    name = "dre-repo"
INFO:   -version = "0.4.3"
INFO:   +version = "0.5.0"
INFO:    description = ""
INFO:    authors = ["DRE Team <dept-DRE@dfinity.org>"]
INFO:    readme = "README.md"
INFO: Patching file Cargo.toml
INFO:   ---
INFO:   +++
INFO:   @@ -28,7 +28,7 @@
INFO:    resolver = "2"
INFO:
INFO:    [workspace.package]
INFO:   -version = "0.4.3"
INFO:   +version = "0.5.0"
INFO:    edition = "2021"
INFO:    authors = ["IC Decentralized Reliability Engineering (DRE) Team"]
INFO:    description = "Tooling for managing the Internet Computer"
INFO: Patching file VERSION
INFO:   ---
INFO:   +++
INFO:   @@ -1 +1 @@
INFO:   -0.4.3
INFO:   +0.5.0
fatal: Needed a single revision
Switched to a new branch 'release-0.5.0'
[release-0.5.0 a14239e5] Release 0.5.0
 4 files changed, 45 insertions(+), 3 deletions(-)
Enumerating objects: 11, done.
Counting objects: 100% (11/11), done.
Delta compression using up to 32 threads
Compressing objects: 100% (5/5), done.
Writing objects: 100% (6/6), 2.18 KiB | 2.18 MiB/s, done.
Total 6 (delta 4), reused 0 (delta 0), pack-reused 0
remote: Resolving deltas: 100% (4/4), completed with 4 local objects.
remote:
remote: Create a pull request for 'release-0.5.0' on GitHub by visiting:
remote:      https://github.com/dfinity/dre/pull/new/release-0.5.0
remote:
remote: GitHub found 16 vulnerabilities on dfinity/dre's default branch (2 critical, 8 high, 6 low). To find out more, visit:
remote:      https://github.com/dfinity/dre/security/dependabot
remote:
To github.com:dfinity/dre.git
 * [new branch]        release-0.5.0 -> release-0.5.0
```

Next, go to the github repo and open the PR, get it approved and merge it.

Next, create and push the git tag to the repo, to trigger the release CI workflow.

```
git checkout main
git pull
git status
./bin/mk-release.py --tag 0.5.0
```

Wait for the triggered [GH action to finish](https://github.com/dfinity/dre/actions). It will take some time because it's building binaries for x86 and for darwin.

Now open the [GH releases page](https://github.com/dfinity/dre/releases).
You should see a new draft release. Edit it, to shorten the release notes by removing entries for older (previous) releases.

Set the `latest` to point to this new release, and finally click publish.

Celebrate!