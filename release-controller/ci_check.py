# TODO:
# validate that realease_notes_ready: false if release doesn't exist on main
# validate that there's no double entries in override_versions
# validate that there's no duplicates in rollout plan
# validate that there are no subnets missing in rollout plan
# validate that there's a stage to update unassigned nodes
# check that all versions within same release have an unique name
# check that all rollout versions (default and override_versions) have valid entries in the release
#   TODO: instead of doing this, we can just halt the rollout if the version is missing
# check that commits are ordered linearly in each release
# check that releases are ordered linearly
# check that previous rollout finished
# check that versions from a release cannot be removed if notes were published to the forum
# check that version exists on ic repo, unless it's marked as a security fix
# validate that excludes_subnets are present on the rollout plan (i.e. valid subnets)
# validate that wait_for_next_week is only set for last stage
# check that version belongs to specified RC

# TODO: additionally consider these
# generate rollout plan to PR if it's different from main branch - how would that look like?
# write all failed validations as a comment on the PR, otherwise just generate a test report in a nice way
# instruct user to remove old RC from the index - we don't need this currently, these versions will be ignored by reconciler in any case

# TODO: other things to consider
# proposed version can be rejected.
