# Submitting firewall change proposals

This guide shows how to propose firewall rule changes using the DRE CLI. It reflects the exact behavior of the `dre firewall` command as implemented in the source code, so the steps and flags below are guaranteed to work.

## Prerequisites
- DRE CLI installed and on your PATH.
- A proposing neuron configured (HSM or private key).
- Access to the target network (e.g., mainnet).

Helpful globals you can pass to any command:
- `--network <mainnet|staging|testnet>`
- `--private-key-pem <path>` or HSM flags
- `--dry-run` to simulate first
- `-y, --yes` to auto-confirm
- `--forum-post-link <URL|discourse|ask|omit>` to control forum posting

See `dre --help` and the Authentication Options in the main README for details.

## Scopes
Rules are stored per scope. The command accepts exactly these scope strings:
- `global`
- `replica_nodes`
- `api_boundary_nodes`
- `subnet(<SUBNET_ID>)`  e.g., `subnet(pjljw-...)`
- `node(<NODE_ID>)`      e.g., `node(z6jp6-...)`

## Inspect current rules (recommended)
```bash
# Example: view current global rules
dre get firewall-rules global | jq

# Or for replica nodes
dre get firewall-rules replica_nodes | jq
```

## Make a change via the interactive editor
Run:
```bash
# Minimal required flags
dre firewall \
  --rules-scope global \
  --summary "<what and why; include forum link if you have it>"
```
Optional but recommended:
- `--title "Proposal to modify firewall rules"` (defaults to that value if omitted)
- `--forum-post-link <URL|discourse|ask|omit>`
- `--dry-run` to simulate; rerun without it to submit
- `--yes` to skip the confirmation prompt, if desired

What happens:
1. The tool fetches the current rules for the given scope and opens your `$EDITOR` with a JSON document.
2. The JSON is a map of `position -> rule`, where positions are integers starting at 0 (order matters). Example structure:
```json
{
  "0": { "...": "existing rule" },
  "1": { "...": "existing rule" }
}
```
3. To add a rule, copy an existing rule object as a template and paste it under a new key equal to the next available integer (append to the end). To update a rule, modify the value at its existing key. To remove a rule, delete its key.

Important details based on the implementation:
- The tool determines your intent by comparing the edited JSON to the original:
  - New key -> addition
  - Same key with different value -> update
  - Removed key -> removal
- Only one type of change is submitted per run (add OR update OR remove). If you mix types, the tool will only submit one batch (the last-by-type after sorting). For multiple types, run the command multiple times.
- Keep the rule object shape consistent with existing rules (field names and types). The safest approach is to copy an existing rule and adjust ports, prefixes, and comment.
- To append to the end, use the next integer position. Do not renumber existing keys unless you intend to update or remove those rules.

## Example: allow DFINITY observability to attest SEV-protected nodes (global)
Goal: open TCP port 19523 to a small set of IPv6 prefixes, at global scope.

1) Preview current rules to find a similar ALLOW rule to copy its field names and valid `action` value:
```bash
dre get firewall-rules global | jq
```
2) Run the interactive editor in dry-run first:
```bash
dre firewall \
  --rules-scope global \
  --title "Allow the DFINITY observability stack to remotely attest SEV-protected nodes" \
  --summary "We propose to allow the DFINITY observability stack to access the remote attestation endpoint on TCP port 19523. This access is essential for monitoring and attesting SEV-protected nodes. It allows the DFINITY observability stack to periodically fetch an attestation report and verify it. For details, see: <FORUM POST URL>" \
  --forum-post-link <FORUM POST URL> \
  --dry-run
```

Example using a local summary file and the `--forum` alias:
```bash
dre firewall \
  --summary "$(cat 2025-10-07-observability-remote-attestation-summary.md)" \
  --rules-scope global \
  --forum https://forum.dfinity.org/t/nns-proposal-enabling-remote-attestation-for-dfinity-observability/58611
```
3) In the JSON that opens, paste a full object. If the ruleset is empty, a minimal valid example is:
```json
{
  "0": {
    "ipv4_prefixes": [],
    "ipv6_prefixes": [
      "2602:fb2b:100:12::/64",
      "2602:fb2b:120:12::/64",
      "2602:fb2b:110:12::/64"
    ],
    "ports": [
      19523
    ],
    "action": 1,
    "comment": "remote attestation for DFINITY obs stack"
  }
}
```
If there are existing entries, keep them as-is and add a new key at the next available integer (e.g., "123"). Use the same `action` value as existing ALLOW rules in your environment.
Notes:
- Copy the `action` value from an existing ALLOW rule in your current ruleset to ensure it matches the expected enum/representation.
- If your environment stores other required fields (e.g., protocol), keep them as in the copied template and adjust as needed.
- Some environments include extra fields such as `user` and `direction`. Preserve them as seen in your existing rules unless you intentionally need to change them.

4) Save and exit the editor. The tool prints a pretty diff and simulates the proposal, computing a content hash.
5) If the simulation looks correct, rerun without `--dry-run`. Add `--yes` to skip confirmation if desired:
```bash
dre firewall \
  --rules-scope global \
  --title "Allow the DFINITY observability stack to remotely attest SEV-protected nodes" \
  --summary "We propose to allow the DFINITY observability stack to access the remote attestation endpoint on TCP port 19523. This access is essential for monitoring and attesting SEV-protected nodes. It allows the DFINITY observability stack to periodically fetch an attestation report and verify it. For details, see: <FORUM POST URL>" \
  --forum <FORUM POST URL>
```

## Tips
- Always dry-run first to review the diff and the generated `ic-admin propose-to-...` command.
- Submit additions, updates, and removals in separate runs.
- Prefer appending new rules to the end unless you have a specific ordering requirement.
- Keep comments clear; include a forum link in the summary to aid reviewers.

## Replica nodes rules: common workflow

Replica nodes (`replica_nodes`) scope is the most commonly changed set of rules. The workflow is identical to the global example above, but you pass `--rules-scope replica_nodes` and copy field shapes from the current replica ruleset.

1) Inspect current replica rules and note the `action` value used for ALLOW and any extra fields:
```bash
dre get firewall-rules replica_nodes | jq
```

2) Run the interactive editor in dry-run first:
```bash
dre firewall \
  --rules-scope replica_nodes \
  --title "Open TCP 19523 for SEV attestation to DFINITY observability" \
  --summary "Allow DFINITY observability prefixes to access TCP 19523 for remote attestation on all replica nodes. See: <FORUM POST URL>" \
  --forum-post-link <FORUM POST URL> \
  --dry-run
```

3) Append a new entry at the next available integer key. Use the same `action` value as other ALLOW rules in the replica ruleset, and preserve optional fields if present:
```json
"<NEXT_INDEX>": {
  "ipv4_prefixes": [],
  "ipv6_prefixes": [
    "2602:fb2b:100:12::/64",
    "2602:fb2b:120:12::/64",
    "2602:fb2b:110:12::/64"
  ],
  "ports": [19523],
  "action": <copy from an existing ALLOW rule>,
  "comment": "remote attestation for DFINITY obs stack",
  "user": null,
  "direction": null
}
```

4) Save and exit to simulate. If correct, rerun without `--dry-run` (optionally with `--yes`). Submit additions, updates, and removals as separate runs if you need multiple kinds of changes.
