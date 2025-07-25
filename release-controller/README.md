# Release controller

Automates the process of proposing new releases for IC HostOS and GuestOS.

## Usage

1. Register new release / version in [release-index.yaml](https://github.com/dfinity/dre/blob/main/release-index.yaml)
  ```yaml
  releases:
    - rc_name: rc--2024-02-21_23-01
      versions:
        # It is customary but not mandatory to add a link to the
        # Qualification pipeline:
        # https://github.com/dfinity/ic/actions/runs/14491317106
        - name: base
          version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
  ```
2. Relevant teams are notified with a link to a Google document for them to review the release notes.  In parallel, placeholder post is created in the forum to prepare for publication of the release notes.
3. Once the Google document is reviewed (all teams crossed out), PR will be created with release notes.
4. Once that PR is merged, the proposal will be placed and the placeholder forum post is updated with the final release notes.
5. Once trusted neurons have voted to adopt the proposal, the adopted release can be rolled out (beyond the scope of release controller).

### Release index reference

Releases are composed of a list of dictionaries, each having (1) an `rc_name` corresponding to the RC branch to be released, and (2) a list of versions each containing a `name` (at least one of which is typically named `base` and corresponds to the first listed version) and a `version` containing the commit ID desired to be tagged and released; two additional version fields `changelog_base` and `security_fix` are documented below.

Out of each version within a release, a release branch named `{rc_name}-{version.name}` will be constructed to create a specific release for GuestOS and (in the case of the `base` or first version of all releases) HostOS.  There is currently no way to force a feature / non-base version of a release to turn into a proposed HostOS release.

Only the two most recent releases will be paid attention to by the release controller.

The release notes (changelog) for each release version is generated automatically, starting from a prior version which is typically determined automatically.  In the case of any base version of a release, the prior base release is considered the baseline for the release notes; in the case of a non-base / feature version, the base version the same release is considered the baseline.

You can override this behavior; a version can have an additional `changelog_base` dictionary with (optional) keys `GuestOS` and/or `HostOS`, whose values must be the name of another release (`rc_name`) listed in the index, as well as the name of one of its versions (typically `base`).  This dictionary allows you to override which release/version combo is used as the baseline for (the start of) the release notes that will be generated for this OS and version combination.  Here is an example:

```yaml
releases:
  - rc_name: rc--2025-05-23_03-21
    versions:
      - name: base
        version: 16825c5cbff83a51983d849b60c9d26b3268bbb6
        changelog_base:
          # Base the changelog for GuestOS at this version onto the May 1st base release.
          # Due to absence of HostOS key, use the normal HostOS baseline detection mechanism
          # for its changelog.
          GuestOS:
            rc_name: rc--2025-05-01_03-23
            name: base
  - rc_name: rc--2025-05-15_03-20
    versions:
      - name: base
        version: 59ad18a77fbeaf3ebbba863972ff20f7ab588d7a
  - rc_name: rc--2025-05-01_03-23
    versions:
      - name: base
        version: f195ba756bc3bf170a2888699e5e74101fdac6ba
```

Finally, changelog generation can be suppressed entirely by adding `security_fix: true` to a version.  This creates an abridged release notes containing no changes at all, and indicating to the users that the code plus the changes will be available at a later date.  Use this flag when a release must be performed from the private security-fixes-only repository, as otherwise the changelog code will not work.

```yaml
releases:
  - rc_name: rc--2025-05-23_03-21
    versions:
      - name: base
        version: 16825c5cbff83a51983d849b60c9d26b3268bbb6
        security_fix: true
```

You are allowed to periodically clean up old releases from the release index, so long as the commit IDs they list are not in the blessed versions for HostOS or GuestOS, and none of the remaining releases refer to them via the `changelog_base` field.  As a rule of thumb, it will not cause problems to delete releases six months or older.

## Recreating notes

Sometimes you'd want to recreate notes, either because a bug occured on the first generation, or you just want to have updated version of the notes submitted.

### Recreate Google Doc

To recreate Google Doc, remove the document from [Google Drive directory](https://drive.google.com/drive/folders/1y-nuH29Gd5Err3pazYH6-LzcDShcOIFf) or rename it such that it doesn't include any release details.

### Recreate GitHub PR with release notes

To recreate GitHub PR, close the outstanding PR and make sure to **delete the branch of the PR**.

## In production

Several credentials are necessary.  For the reconciler:

1. The Google Drive credentials.  These are stored in the DRE Team vault and named *FIT on-call schedule sync and release controller Google Drive credential*.  Saved to a file, the path to this file must be set in environment variable `GDOCS_CREDENTIALS_PATH`.
2. `PROPOSER_NEURON_ID` should be set to the neuron ID of the proposer, and the proposer key material should be saved to a file (asn the DRE team for information), whose path should be added to environment variable `PROPOSER_KEY_FILE`.
3. The `GITHUB_TOKEN` environment variable must be set to the token that has access to the IC and DRE repositories.  This secret resides in the DRE Team vault under the name *Release Controller GitHub API Key*.  In the reconciler, this token is used to push tags.

The commit annotator only needs the `GITHUB_TOKEN` credentials.  This token is used to push notes.

## Contributing

The project is split into two parts - commit annotator and reconciler.

If you want fix a bug, or add a feature, please consider writing a test.

### commit annotator

This simple service checks out each commit on `master` and `rc-*` branches of IC repo and runs [target-determinator](https://github.com/bazel-contrib/target-determinator) to identify whether GuestOS build changed as a result of changes in that commit. This information is then pushed to git notes and later used by reconciler.

The annotator has a bonus mode to manually annotate failing commits.  See below for more info.

### reconciler

Reconciler is responsible for generating release notes (1), publishing them as google docs and sending a notification to #eng-release Slack channel (2), creating a GitHub PR to publish notes (3), placing the proposal for electing a version (4) and creating and updating forum post (5).

1. generating release notes
  Done by release_notes.py. You can manually run the script for debugging purposes.

2. google docs publish
  Done by google_docs.py. You can run the program manually to debug issues. Change the `main()` function to your needs.

3. creating a GitHub PR to publish notes
  Done by publish_notes.py. It's not recommended to run this manually. Instead, if you have an issue, try to create a unit test to resolve the issue. You can download the Google Doc you're having problems with to use it in your test. See tests that use `release-controller/test_data/b0ade55f7e8999e2842fe3f49df163ba224b71a2.docx`.

4. placing the proposal for electing a version
  Done by dre_cli.py / reconciler.py. There should be a logs for the command that was run if you want to debug any issues with it.

5. forum post update
  Done by forum.py. You can run the program manually to debug issues. Change the `main()` function to your needs.

  It's important to note that forum logic depends on finding alredy created blog posts by querying posts from authenticated user (@DRETeam). For those reasons, it won't be able to find manually created posts by other users.

## Resolving issues

### Diagnostics

The release controller [has its own dashboard](https://grafana.ch1-rel1.dfinity.network/d/release-controller/release-controller).
Use the dashboard to supervise the progress of the components that comprise the release controller.

### Google Docs generation was wrong for particular commit

This could happen if release-index.yaml was wrongly configured or if there's a major bug that needs to be adressed before regenerating the notes again.

#### Resolution

1. To cause the reconciler to regenerate release notes: move the document outside of the Google Drive folder. Renaming it to something meaningless (e.g. to-delete-12-09-24) should also do the trick.
2. To fix the code in the commit annotator: manually generate the release notes (see below) on your computer and make changes to the queries or the code until the notes look as expected.

### Release notes are not yet ready for a long time

This is caused by one or more missing GuestOS / HostOS annotations.  Release controller is stuck generating release notes because it's missing a note for some commit.

#### Diagnostics

Verify that the annotator has completed and is not stuck on any branch (use the dashboard listed above).

If `target-determinator` is crashing as the annotator executes it, here is how you find the failing commit causing the crash:

1. Click on the Custom query square on the title of the *Combined annotater and reconciler logs* pane on the dashboard.
2. Search for `annotate_object` -- the most recent occurrence will have the failing commit ID.

#### Resolution

If the problem is that the annotator keeps crashing because a commit is not buildable, you can manually annotate that commit and that will cause the annotator to skip it.  There is an example below on how to manually annotate a failing commit.

You may have to annotate all commits not annotated prior to the failing commit as well (although that should not be necessary because the annotator generally annotates from oldest to newest commit, so all older commits should already be annotated).


> [!TIP]
> If someone messed up and labeled commits in between, commit annotator might report that it labeled everything when it did not, and reconciler may never be ready with the release notes.
> Run the below commands on IC repo to find gaps where there are commits without labels.

```shell
git fetch origin 'refs/notes/*:refs/notes/*' -f --prune
git log --format='%H' --no-merges $BASE_COMMIT..$RELEASE_COMMIT | xargs -L1 -I_commit bash -c "echo -n '_commit '; git notes --ref guestos-changed show _commit | cat"
# substitute guestos-changed with hostos-changed to detect gaps in HostOS annotations.
# substitute guestos-changed with guestos-targets to see targets and target-determinator output for that commit's annotation work..
```

### Missing proposal

Proposal placement most likely failed.


#### Evaluation

release-controller should have a warning message something like this:

```
"version 99ab7f03700ba6cf832eb18ffd55228f56ae927a: earlier proposal submission attempted but most likely failed"
```

Make sure also that few minutes have passed and that public dashboard still doesn't list the proposal.  Sometimes it takes a minute or two.

If the proposal was indeed submitted, you don't have to do anything -- the reconciler will notice and continue normally.

#### Resolution

> [!WARNING]
> Should resolve by itself in newer versions

1. Top up the release-controller neuron if needed
2. Execute into the pod
  ```
  kubectl -n release-controller exec -it deployment/release-controller -- bash
  ```
3. Delete the state
  ```
  rm /state/<full_commit_hash>
  ```

## Development

Please see the parent folder's `README.md` for virtual environment setup.
Follow the whole *Contributing* section to the letter.

### Running the reconciler in dry-run mode

```sh
bazel run //release-controller:release-controller -- --dry-run --verbose
```

No credentials of any kind are required by this mode.  By default everything the
reconciler does in this mode has no outward effect.

All the operations it executes are volatile as well.

If you want the release notes this mock mode stores to be persisted in a folder
so they are not regenerated on every run:

```sh
export RECONCILER_DRY_RUN_RELEASE_NOTES_STORAGE=/tmp/dryrun/relnotes
bazel run //release-controller:release-controller \
  --action_env=RECONCILER_DRY_RUN_RELEASE_NOTES_STORAGE \
  -- --dry-run --verbose
```

If you want the mock forum interactions to be remembered between runs:

```sh
export RECONCILER_DRY_RUN_FORUM_STORAGE=/tmp/dryrun/forum
bazel run //release-controller:release-controller \
  --action_env=RECONCILER_DRY_RUN_FORUM_STORAGE \
  -- --dry-run --verbose
```

Typing errors preventing you from running it, because you are editing code and
testing your changes?  Add `--output_groups=-mypy` right after `bazel run`.

The optional argument `--skip-preloading-state` makes it so that the reconciler
will not preload its list of known proposals by version from the governance
canister.  It is useful (in conjunction with an empty reconciler state folder)
to make the reconciler do all the work of submitting proposals again.  It should
only be used alongside `--dry-run`, to avoid submitting proposals twice.

### Running the reconciler in the container it ships

You can load the reconciler into your local podman or docker system:

```sh
bazel run //release-controller:oci_image_load
```

This will spit out a SHA256 sum, which is the name of the container image just
built and imported into your containerization system.  Run it as follows:

```sh
SHASUM=...
podman run --rm -it --entrypoint=/release-controller/release-controller $SHASUM
```

Or, in short:

```sh
mkdir -p -m 0777 /tmp/git
podman run --rm -it \
  -v /tmp/git:/root/.cache \
  --entrypoint /release-controller/release-controller \
  $(bazel run --verbose_failures //release-controller:oci_image_load | tail -1 | cut -d : -f 3)
```

### Running the annotator locally in "dry-run mode"

The annotator can be run in a mostly stateless mode, for one single loop,
with the following options:

```sh
bazel run //release-controller:commit-annotator \
  -- \
  --no-push-annotations \
  --loop-every=0 \
  --no-fetch-annotations \ # don't clobber locally created annotations 
  --verbose
```

The annotator can also be run as a podman container, with a similar
technique as above.  However, the annotator requires `--user $UID`
because Bazel will not run as root (UID 0).

Please consult `--help` for additional options.

### Manually annotate a troublesome commit

```sh
export GITHUB_TOKEN=<any Github token with push access to the IC repo>
COMMIT_TO_ANNOTATE=9da8cc52d3d576410174bb28d629862f05a635e0
AFFECTS_OS=yes
WHICH_OS=HostOS # or GuestOS, or leave out --os-kind for all OSes
bazel run //release-controller:commit-annotator \
  -- \
  manually-annotate \
  $COMMIT_TO_ANNOTATE $AFFECTS_OS --os-kind $WHICH_OS
```

### Generate release notes locally

Release notes can be generated locally, using the following command:

```sh
PREV_RC=rc--2025-03-27_03-14-base
PREV_COMMIT=3ae3649a2366aaca83404b692fc58e4c6e604a25
CURR_RC=rc--2025-04-03_03-15
CURR_COMMIT=68fc31a141b25f842f078c600168d8211339f422
bazel run //release-controller:release-notes -- \
   $PREV_RC $PREV_COMMIT $CURR_RC $CURR_COMMIT \
  --verbose
```

The form of the command above requires you to run a commit annotator in
parallel.  If you want to use the internal commit annotator that does not
need a commit annotator running in parallel, add option
`--commit-annotator-url local` instead.  If you want to *recalculate* the
commit annotations instead of using cached ones, you can use option
`--commit-annotator-url recreate`.  This last option is useful when
testing the effects of changes made to the commit annotator code or Bazel
query formulas the annotator uses.

A great tip / trick to diagnose exactly what the release notes and
commit annotation processes would do is to pick a commit from the IC
repo, figure out which its parent commit is, then run:

```sh
PREV_RC=prev
PREV_COMMIT=1354f31c9cd4fb6b4a65ab64eb9ac4a0a4d16839 # parent commit
CURR_RC=curr
CURR_COMMIT=f8131bfbc2d339716a9cff06e04de49a68e5a80b # commit
bazel run //release-controller:release-notes -- \
   $PREV_RC $PREV_COMMIT $CURR_RC $CURR_COMMIT \
   --commit-annotator-url recreate \
  --os-kind=GuestOS \
  --verbose
bazel run //release-controller:release-notes -- \
   $PREV_RC $PREV_COMMIT $CURR_RC $CURR_COMMIT \
   --commit-annotator-url recreate \
  --os-kind=HostOS \
  --verbose
```

That run tells you what the annotation process would do for that single
commit in question.

Please consult `--help` for additional options.

### Tests

#### Unit tests

```sh
bazel test //release-controller/...
```

The above runs all tests and typechecks tested files.

With a `.venv` setup by `rye`, you can also run (with varying levels of success):

```sh
export PYTHONPATH=$PWD/release-controller/
.venv/bin/python3 release-controller/tests/runner.py
```

If you want to run a specific test file, specify its path as an argument
to the above command line.

#### Typing correctness

Building it all tests MyPy types:

```sh
bazel build //release-controller/...
```

### Maintenance

The container image currently used by release controller components
is an Ubuntu 24.04 image built by Bazel.  Refer to [BUILD.bazel](./BUILD.bazel)
and [../images/BUILD.bazel](../images/BUILD.bazel) for instructions
on how to maintain and update the images.
