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
