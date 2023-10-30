#!/usr/bin/env python3
"""
Script for calculating IPv6 addresses based on a deterministic MAC address and IPv6 prefix.

Example usage:

❯ ./mainnet_ipv6_calc.py 7cc255778108 2a0a:5000:0:3
IPv6 host: 2a0a:5000:0:3:6800:2cff:fe9c:444f
IPv6 guest: 2a0a:5000:0:3:6801:2cff:fe9c:444f
"""

import hashlib
import ipaddress
import logging
import re
import sys


def calculate_deterministic_mac(deployment: str, management_nic_mac: str, guest_index: int):
    """Calculate and return the deterministic MAC address based on the deployment, management NIC, etc."""
    mac_lower = management_nic_mac.lower().replace(":", "")
    # split mac_lower into 2 character long strings and join them by colons
    mac_lower = ":".join([mac_lower[i : i + 2] for i in range(0, len(mac_lower), 2)])

    p = re.compile(r"^(?:[0-9a-fA-F]:?){12}$")
    if not p.match(mac_lower):
        logging.error(
            "The provided MAC address of the management NIC %s is invalid. "
            "Please enter six octets separated by colons.",
            mac_lower,
        )
        sys.exit(1)
    # The bash script used to prepare the seed with a trailing newline, so we do the same thing here
    seed = mac_lower + deployment + "\n"
    # Take the first 8 chars for the sha256 sum of the seed
    vendor_part = hashlib.sha256(seed.encode("utf-8")).hexdigest()[:8]

    version_octet = "6a"
    deterministic_mac = f"{version_octet}0{guest_index}{vendor_part}"
    # chunk into groups of 2 and re-join. 6a015c532060 => 6a:01:5c:53:20:60
    return ":".join([deterministic_mac[i : i + 2] for i in range(0, len(deterministic_mac), 2)])


def calculate_deterministic_ipv6(mac_addr: str, ipv6_prefix: str):
    """Calculate and return the IPv6 address based on the deterministic MAC, and IPv6 prefix/subnet."""
    # For an example mac_addr 6a:01:5c:53:20:60 => 6a015c532060
    output = mac_addr.replace(":", "")
    # inject fffe after the first 6 bytes => 6a015cfffe532060
    output = f"{output[:6]}fffe{output[6:]}"
    # ensure that bit 2 of the top-most 2 bytes (half-word) is XOR-ed with 1 (switch value). 6a => 68
    top_half_word = int("0x" + output[:2], 16) ^ 2
    top_half_word_str = hex(top_half_word)[2:]  # convert to a hex string
    # concat the transformed top-most 2 bytes with the rest. 68 + 015cfffe532060 => 68015cfffe532060
    output = f"{top_half_word_str}{output[2:]}"
    # chunk into groups of 4 and re-join. 68015cfffe532060 => 6801:5cff:fe53:2060
    output = ":".join([output[i : i + 4] for i in range(0, len(output), 4)])

    # join with the ipv6 prefix and generate a compressed representation.
    # => "2a02:41b:300e:0" + ":" + "6801:5cff:fe53:2060"
    ipv6_compressed = ipaddress.ip_address(f"{ipv6_prefix}:{output}")
    return ipv6_compressed


def parse_args():
    """Parse and return CLI arguments."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Calculate IPv6 addresses based on a deterministic MAC address and IPv6 prefix."
    )
    parser.add_argument(
        "--deployment",
        type=str,
        default="mainnet",
        help="The target IC deployment.",
    )
    parser.add_argument(
        "mac_addr",
        type=str,
        help="The BMC MAC address on the machine.",
    )
    parser.add_argument(
        "ipv6_prefix",
        type=str,
        help="The IPv6 prefix for the Data Center.",
    )
    return parser.parse_args()


def main():
    """Main entry point."""
    args = parse_args()
    ipv6_prefix = args.ipv6_prefix
    if ipv6_prefix.endswith("/64"):
        ipv6_prefix = ipv6_prefix[:-3]
    if ipv6_prefix.endswith("::"):
        ipv6_prefix = ipv6_prefix[:-2]
    gen_mac_host = calculate_deterministic_mac(deployment=args.deployment, management_nic_mac=args.mac_addr, guest_index=0)
    gen_mac_guest1 = calculate_deterministic_mac(deployment=args.deployment, management_nic_mac=args.mac_addr, guest_index=1)
    print("IPv6 host:", calculate_deterministic_ipv6(gen_mac_host, ipv6_prefix))
    print("IPv6 guest:", calculate_deterministic_ipv6(gen_mac_guest1, ipv6_prefix))


if __name__ == "__main__":
    main()
