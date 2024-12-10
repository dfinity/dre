# How to run qualification

There are three ways to run the test and a couple of environment variables that a user should be aware of.

??? tip "Suggested way of running"
    Since this is a long test and bazel tends to be pretty heavy on the system it is proposed to run it on a devenv!
    To do that you should follow the guide on how to create a devenv by idx and once its deployed you can:
    ```bash
    eval `ssh-agent -s`
    ssh-add ~/.ssh/ssh_key_that_has_access_to_k8s_repo
    ssh devenv
    cd /to/root/of/ic/repo
    ./ci/container/container-run.sh
    ...
    ```

## Running using `ict`
`ict` is a go-lang tool that was developed to help developers run tests with less friction in having to write long test names. To use `ict` one can spin up a new shell and:
```bash
cd /to/root/of/ic/repo
./ci/container/container-run.sh
ict test guest_os_qualification -- --test_timeout=7200 --keep_going
```

## Running using `bazel test`
Spin up a new shell and:
```bash
cd /to/root/of/ic/repo
./ci/container/container-run.sh
bazel test //rs/tests/dre:guest_os_qualification --config=systest --cache_test_results=no --test_env=IC_DASHBOARDS_DIR=/path/to/k8s_repo/bases/apps/ic-dashboards --sandbox_add_mount_pair=/path/to/k8s_repo/bases/apps/ic-dashboards --test_timeout=7200 --keep_going
```

## Environment variables
- `OLD_VERSION`: specifies the starting version for a testnet. If its not specified it will default to the version specified in `tests/mainnet_revision.json`.

## How does it work
Qualification test consits of multiple steps. If we have versions `A` and `B` where `A` is already deployed to the network and `B` is a version that is being qualified the steps would look like the following:

1. Ensure that version `A` is on all subnets
2. Ensure that version `A` is on all unassigned nodes
3. Upgrading phase:

    1. Deploy version `B` to application subnets
    2. Deploy version `B` to system subnets
    3. Deploy version `B` to unassigned nodes

4. Testing phase:

    1. Run performance tests
    2. Run xnet tests

5. Downgrade phase:

    1. Deploy version `A` to application subnets
    2. Deploy version `A` to system subnets
    3. Deploy version `A` to unassigned nodes

6. Testing phase:

    1. Run performance tests
    2. Run xnet tests
