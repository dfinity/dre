"""
rules for creating oci images from rust binaries
"""

load("@rules_oci//oci:defs.bzl", "oci_image", "oci_push", "oci_tarball")
load("@rules_pkg//:pkg.bzl", "pkg_tar")

def rust_binary_oci_image_rules(name, src, base_image = "@distroless_cc_debian12", other_layers = []):
    """macro for creating oci image from rust binary

    Args:
        name: not used
        src: label of rust binary to be put in the OCI image
        base_image: base image for building rust binaries
        other_layers: optional of other layers to be added, e.g. deb packages
    """
    binary = native.package_relative_label(src)
    tar_rule_name = "{}_layer".format(binary.name)
    pkg_tar(
        name = tar_rule_name,
        srcs = [binary],
    )

    image_rule_name = "{}-image".format(binary.name)
    oci_image(
        name = image_rule_name,
        # Consider using even more minimalistic docker image since we're using static compile
        base = base_image,
        entrypoint = ["/{}".format(binary.name)],
        tars = [tar_rule_name] + other_layers,
    )

    tarball_name = "tarball".format(binary.name)
    oci_tarball(
        name = tarball_name,
        image = image_rule_name,
        repo_tags = ["localhost/{}:latest".format(binary.name)]
    )

    oci_push(
        name = "push_image",
        image = image_rule_name,
        repository = "ghcr.io/dfinity/dre/{}".format(binary.name),
    )
