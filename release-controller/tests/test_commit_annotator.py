import itertools
import logging
import pytest

from commit_annotation import (
    COMMIT_BELONGS,
    COMMIT_DOES_NOT_BELONG,
    GitRepoAnnotator,
)
from commit_annotator import compute_annotations_for_object
from const import GUESTOS
from tests.fixtures import (
    ic_repo as ic_repo,
    annotator as annotator,
)


_LOGGER = logging.getLogger()


def _test_guestos_changed(
    annotator: GitRepoAnnotator, object: str, expect_changed: bool
) -> None:
    targets, determinator_output, belongs = compute_annotations_for_object(
        object=object,
        os_kind=GUESTOS,
        annotator=annotator,
    )
    expected = COMMIT_BELONGS if expect_changed else COMMIT_DOES_NOT_BELONG
    assert belongs == expected, "%r != %r, targets affected by commit %s:\n%s" % (
        belongs,
        expected,
        object,
        determinator_output,
    )


target_determinator_cycle = itertools.cycle("abcde")


# Marks on tests attempt to limit parallelism on multiple target-determinators
# running simultaneously.
@pytest.mark.xdist_group(name=f"target_determinator_{next(target_determinator_cycle)}")
def test_guestos_changed__registry_changed(annotator: GitRepoAnnotator) -> None:
    """Registry changes impact GuestOS and therefore should be included."""
    _test_guestos_changed(
        annotator,
        # feat(registry): Added `maybe_chunkify`... (#4751)
        object="ca2d5e7dfc8a70f6998d9edd35c0d020922fe829",
        expect_changed=True,
    )


@pytest.mark.xdist_group(name=f"target_determinator_{next(target_determinator_cycle)}")
def test_guestos_changed__docs_changed(annotator: GitRepoAnnotator) -> None:
    """Simple documentation changes do not impact GuestOS."""
    _test_guestos_changed(
        annotator,
        # chore: Add contributing guide for management canister APIs (#4852)
        object="a8774ac3b1172678554857bdf23c33cf913dde1d",
        expect_changed=False,
    )


@pytest.mark.xdist_group(name=f"target_determinator_{next(target_determinator_cycle)}")
def test_guestos_changed__bumped_dependencies(annotator: GitRepoAnnotator) -> None:
    """Dependency bump known to be dependency of GuestOS affects GuestOS."""
    _test_guestos_changed(
        annotator,
        # chore(EXC-2013): upgrade wasmtime to v.31 (#4673)
        object="8c1f8b0d3060f5f905d42bf68eb36ac6130c4b10",
        expect_changed=True,
    )


@pytest.mark.xdist_group(name=f"target_determinator_{next(target_determinator_cycle)}")
def test_guestos_changed__canister_not_in_replica_changed(
    annotator: GitRepoAnnotator,
) -> None:
    """Changes in a canister not shipped with replica should not affect GuestOS."""
    _test_guestos_changed(
        annotator,
        # feat(ckbtc): Add get_utxos_cache to reduce latency of update_balance calls (#4788)
        object="9204403648ccb03c3c65f86e55864f5cbbbf5059",
        expect_changed=False,
    )


@pytest.mark.xdist_group(name=f"target_determinator_{next(target_determinator_cycle)}")
def test_guestos_changed__cargo_lock_paths_only(
    annotator: GitRepoAnnotator,
) -> None:
    "Minor changes only to Cargo.lock that don't affect the replica should not affect GuestOS."
    _test_guestos_changed(
        annotator,
        # fix: match cloudflare's crate rev in Cargo.toml with external_crates.bzl (#4874)
        object="a6528fc55695b6ae4d330f50088d9b4e1f0714f1",
        expect_changed=False,
    )
