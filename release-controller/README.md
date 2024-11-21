# Release controller

Automates parts of the process of proposing new releases for

## Usage

1. Register new release / version
  ```yaml
  releases:
    - rc_name: rc--2024-02-21_23-01
      versions:
        - name: baseline
          version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
  ```

2. Once the Google Doc is finished (all teams crossed out), PR will be created with release notes. Once it's merged, the proposal will be placed and forum post updated.

## Recreating notes

Sometimes you'd want to recreate notes, either because a bug occured on the first generation, or you just want to have updated version of the notes submitted.

### Recreate Google Doc

To recreate Google Doc, remove the document from [Google Drive directory](https://drive.google.com/drive/folders/1y-nuH29Gd5Err3pazYH6-LzcDShcOIFf) or rename it such that it doesn't include any release details.

### Recreate GitHub PR with release notes

To recreate GitHub PR, close the outstanding PR and make sure to **delete the branch of the PR**.

## Contributing

The project is split into two parts - commit annotator and reconciler.

If you want fix a bug, or add a feature, please consider writing a test.

### commit annotator

This simple service checks out each commit on `master` and `rc-*` branches of IC repo and runs [target-determinator](https://github.com/bazel-contrib/target-determinator) to identify whether GuestOS build changed as a result of changes in that commit. This information is then pushed to git notes and later used by reconciler.

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

### Google Docs generation was wrong for particular commit

This could happen if release-index.yaml was wrongly configured or if there's a major bug that needs to be adressed before regenerating the notes again.

#### Resolution

Simply move the document outside of the Google Drive folder. Renaming it to something meaningless (e.g. to-delete-12-09-24) should also do the trick.

### Missing GuestOS label

Release controller is stuck generating release notes because it's missing a GuestOS label for some commit.

#### Evaluation

Error message in release-controller should look something like this.

```
ValueError: Could not find targets for commit 99ab7f0370
ERROR:root:failed to reconcile: Could not find targets for commit 99ab7f0370
```

To resolve, you'll need to manually label all the commits that commit-annotator is struggling with.

> [!CAUTION]
> Do not annotate commits reported by release-controller!
> This will break commit-annotator and then you'll end up having to annotate more commits manually.
> Instead, see if commit-annotator is stuck on certain commit and annotate that one.

Example error message in commit-annotator.

```
INFO:root:annotating git commit 7d81b536b2f66fd779198e2e4dbb405381545a55
ERROR: ...
```

#### Resolution

1. Fetch git notes
  ```
  git fetch origin 'refs/notes/*:refs/notes/*' -f --prune
  ```

2. Display notes for a commit. This should normally output `True` or `False`.
  ```
  git notes --ref guestos-changed show <commit>
  ```

3. If you get an error, you can manually label the commit.
  ```
  git notes --ref guestos-changed add -m "<True|False>" <commit>
  ```

4. Finally, push the labels. If you get a conflict at this point, just start over instead of force pushing.
  ```
  git push origin refs/notes/guestos-changed
  ```

> [!TIP]
> If someone messed up and labeled commits in between, commit annotator might report that it labeled everything.
> Run the below commands on IC repo to find commits without labels.

```shell
git fetch origin 'refs/notes/*:refs/notes/*' -f --prune
git log --format='%H' --no-merges [BASE_COMMIT]..[RELEASE_COMMIT] | xargs -L1 -I_commit bash -c "echo -n '_commit '; git notes --ref guestos-changed show _commit | cat"
```


### Missing proposal

Proposal placement most likely failed.


#### Evaluation

release-controller should have a warning message something like this:

```
"version 99ab7f03700ba6cf832eb18ffd55228f56ae927a: earlier proposal submission attempted but most likely failed"
```

Make sure also that few minutes have passed and that public dashboard still doesn't list the proposal.

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
