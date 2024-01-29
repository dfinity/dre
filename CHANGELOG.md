## 0.2.1

### CI
- Cleanup and reorganize ci config
- Update dependencies and bazel cache in nightly jobs, a few times per week
- Enable grouping of dependabot PRs (#101)

### Feat

- dashboard: Automatically update the internal dashboard k8s deployment on merges to main
- release-notes: Migrate the release notes script to the DRE repo (#119)

### Fixes

- bazel: Update build configuration and version information (#121)
- dashboard: go back to the bitnami git image as distroless has no git (#112)

### Chores

- bump mkdocs-material from 9.5.4 to 9.5.6 (#123)
- bump clickhouse-connect from 0.6.23 to 0.7.0 (#125)
- bump black from 23.12.1 to 24.1.1 (#124)
- bump ansible from 8.7.0 to 9.1.0 (#84)

### Docs

- Update the readme and some more docs (#117)
- Add some tips and tricks for our k8s ops (#115)
- XDR explanation (#88)
- How to get wallet ID when the canister has been created earlier (#89)
- Update the links for running the trustworthy metrics notebook online (#92)

### IC Observability
- Service discovery: Fixing put to do correct validation and correct backup restoring (#126)
- Simplify the definition management code in multiservice-discovery. (#116)
- Scrape GuestOS metrics-proxy and clean up issues in multiservice-discovery
- Add test for multiservice-discovery (#100)

## 0.2.0

### CI
- Automatically push containers if branch name starts with "container" (#65)
- perf(ci): Reduce the bazel cache size (#56)
- feat(ci): Make github releases on git tag push (#53)
- updating image refs to use non distroless images (#54)

### Docs
- Deploy mkdocs to github pages (#66)
- Add various docs, including those around Trustworthy Node Metrics

### Scripts
- Adding missing script for creating tables in clickhouse (#72)
- feat(k8s): Moving k8s python scripts to DRE repo (#70)

### Dashboard
- Dashboard: Enable searching by operator principal (#74)

### Observability
- Service Discovery: Avoid returning empty list of targets on startup (#62)
- Accept invalid certs (#61)

### Chores
- chore: Consolidate and deduplicate cargo deps (#64)
- chore(deps): bump the npm_and_yarn group across 2 directories with 1 update (#57)
- Remove the "ic" submodule
- Bump @adobe/css-tools from 4.3.1 to 4.3.2 in /dashboard (#22)
- chore(deps): bump the pip group across 1 directories with 2 updates (#55)


## 0.1.0

- Initial public release
- The tooling from this repo can help for Internet Computer operations: querying current state, submitting NNS proposals, etc.
