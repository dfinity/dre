# API Boundary Node Incident Handling

The DFINITY Foundation proposes adding functionality to configure temporary rate limits at the API boundary nodes to protect vulnerable components during incidents. This feature is especially important for supporting [the new boundary node architecture](https://forum.dfinity.org/t/boundary-node-roadmap/15562), in particular the API boundary nodes. Rate limits would be only activated as part of incident response under the [“Security Patch Policy and Procedure,”](https://dashboard.internetcomputer.org/proposal/48792) which the NNS adopted in March 2022.

This proposal was previously discussed on the forum on Oct 21, 2024: [https://forum.dfinity.org/t/incident-handling-with-the-new-boundary-node-architecture/36390](https://forum.dfinity.org/t/incident-handling-with-the-new-boundary-node-architecture/36390)

## Background

In certain incidents, it may be necessary to temporarily block or throttle requests to safeguard vulnerable components, which can range from specific canister methods to entire subnets. These rate limits are temporary patches and are removed once a permanent fix is implemented. Given that the new API boundary nodes are fully NNS-controlled, there must also be an NNS-controlled mechanism to set such rate limits.

## Proposal

We propose the creation of a canister to store rate-limiting rules and enforce them across all API boundary nodes. This canister will be controlled by the NNS, including the authorization of specific principals to configure rate-limiting rules. During an incident, active rate-limiting rules will remain private to prevent the exposure of vulnerabilities. Once the incident is resolved, these rules will be made public as part of the post-mortem.

If this proposal is approved, the DFINITY Foundation will initiate all required steps to set up the canister and incorporate this functionality into the incident handling process. Additional proposals will follow to authorize the canister installation and configuration.