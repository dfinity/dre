# Release process

Getting the version deployed to mainnet

1. Register new release

```yaml
releases: # add new release in step 2
  - rc_name: rc--2024-02-21_23-01
    versions:
      - name: initial
        version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        release_notes_ready: false
        default: true
```

2. When the notes are ready, flip the `release_notes_ready` flag.

```diff
    releases: # add new release in step 2
    - rc_name: rc--2024-02-21_23-01
        versions:
        - name: initial
            version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
+           release_notes_ready: true
-           release_notes_ready: false
        default: true
```

3. Approve the release notes from automatically created PR.

4. Change the rollout release and modify rollout plan if needed.

```yaml
rollout:
  deploy_release:
    name: rc--2024-02-21_23-01
```

After the PR is merged, the rollout is now running! 🎉

To intervene in the rollout, take a look at different scenarios below.

## Scenarios

### Pausing the rollout

If there's any potential issues with the network, you might want to pause the rollout to identify if the issue is caused by the rollout of the new version.

```yaml
rollout:
  pause: true
```

### Manually mark the subnet as healthy

TODO:

### Releasing a feature

```diff
  rollout:
    deploy_release:
      name: rc--2024-02-21_23-01
      versions:
        default: initial
+       overrides:
+         - name: p2p
+           subnets:
+             - shefu
+             - qdvhd
+             - bkfrj
```

### Hotfixing

Rollout will first apply all the fixes before continuing with the rollout.

Your configuration will look something like this. Remember to follow the same process for releasing a version as described at the beginning.

Consider to [pause](#pausing-the-rollout) the rollout before starting the hotfix process.

```yaml hl_lines="3-5"
releases:
  - rc_name: rc--2024-02-21_23-01
    versions:
      - name: initial
        version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        release_notes_ready: true
        default: true
      - name: hotfix-initial
        version: 6c2921d320602b01aa038812f5309dedaa693f80
        release_notes_ready: true
        hotfix:
          version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
```

#### Excluding some subnets

Sometimes we don't want to publish hotfixes to some subnets either because they won't have any effect and/or is too risky.

```yaml
      - name: hotfix-initial
        version: 6c2921d320602b01aa038812f5309dedaa693f80
        release_notes_ready: true
        hotfix:
          version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
          excludes_subnets:
            - tdb26
```

#### Security hotfix

Since the security hotfix do not have release notes, release controller won't and should not be able to generate the notes for such release. In the case, you need to mark the version as a security fix. Please note that this is only possible for hotfix versions.

```yaml
      - name: hotfix-initial
        version: 6c2921d320602b01aa038812f5309dedaa693f80
        release_notes_ready: true
        hotfix:
          version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
          security: true
```
