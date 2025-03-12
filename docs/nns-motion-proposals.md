# Submitting NNS Motion Proposals

This guide explains how to submit motion proposals to the Network Nervous System (NNS) using the DRE CLI tool, based on the actual implementation.

## Prerequisites

- DRE CLI tool installed
- A neuron with sufficient voting power
- Required proposal submission fee
- Forum discussion with community feedback (at least 2 weeks old)
- Authentication credentials (private key or HSM)

## Forum Discussion Requirements

Before submitting a motion proposal, it is crucial to:

1. **Create a Forum Discussion**
    - Post your proposal on the official Internet Computer forum
    - Include all relevant details and documentation
    - Allow sufficient time for community feedback

2. **Discussion Period**
    - A minimum of 2 weeks of discussion is expected
    - This period allows the community to:
        - Review the proposal
        - Provide feedback
        - Suggest improvements
        - Raise concerns

3. **Forum Link Requirement**
    - A link to the forum discussion is mandatory
    - Proposals without forum discussion links will likely be rejected
    - The discussion should show active engagement with community feedback

!!! warning "Important"
    Proposals submitted without adequate forum discussion (less than 2 weeks) or without a forum link are likely to be rejected. Ensure you have met these requirements before submission.

## Authentication Options

### Private Key Authentication
```bash
--private-key-pem <PATH>  # Path to private key file (PEM format)
```

### HSM Authentication
```bash
--hsm-pin <PIN>          # Pin for the HSM key
--hsm-slot <SLOT>        # Slot that HSM key uses
--hsm-key-id <KEY_ID>    # HSM Key ID
```

## Proposal Components

A motion proposal consists of:

1. **Summary** (required, max 30 KiB)
    - Written in Markdown format
    - Can include a title as first-line H1 heading
    - Main content describing the proposal

2. **Title** (optional)
    - Can be specified explicitly or extracted from summary
    - If not specified, extracted from first H1 heading in summary

3. **Motion Text** (optional, max 100 KiB)
    - Detailed description of the motion
    - If not provided, defaults to referencing the summary

## Command Line Usage

### Basic Command Structure
```bash
dre governance propose motion [OPTIONS] <SUMMARY_FILE>
```

### Required Arguments
- `<SUMMARY_FILE>`: Path to the proposal summary file (max 30 KiB)
    - Use "-" to read from standard input
    - First line can be H1 heading for title (it can start with `# `, for example: `# Title`)

### Essential Options
```bash
--neuron-id <ID>         # Neuron ID for proposal submission, should not be necessary to be set explicitly
--title <TITLE>          # Optional custom title
--motion-text-file <FILE># Motion text file (max 100 KiB)
--motion-text <TEXT>     # Direct motion text input
```

### Forum Integration Options
```bash
--forum-post-link <OPTION>  # Forum link handling:
                           # - 'discourse': Auto-create post/topic
                           # - URL: Direct forum post link
                           # - 'ask': Prompt for link
                           # - 'omit': Skip link (discouraged)
```

### Discourse Forum Integration, for automatic post creation

Note that this requires a special API key, which may not be available to you.

```bash
--discourse-api-key <KEY>   # API key for forum interaction
--discourse-api-user <USER> # API user (default: DRE-Team)
--discourse-api-url <URL>   # Forum URL (default: https://forum.dfinity.org)
```

Also, due to the requirement that forum posts have long-lasting discussion before proposal submission, this type of discourse integration is not very useful for this type of proposals.

### Additional Options
```bash
--dry-run              # Simulate proposal submission
--verbose              # Print detailed information
-y, --yes              # Skip confirmation prompt
```

## Motion Proposal Workflow for DRE Team

1. Prepare your motion proposal content in Markdown, similar to other files in `/docs/motion-proposals`.
2. Create a new file under the repository path /docs/motion-proposals/<filename>.md (the filename can initially be descriptive, for example "proposal-summary.md").
3. Open a pull request (PR) with your changes and get feedback from others.
4. Address any feedback by making changes in the PR until the proposal content is approved.
5. Submit the proposal using the DRE CLI tool, following the process described below.
6. After submission, update the filename to include the submitted proposal ID (for example, change `proposal-summary.md` to `<proposalID> proposal-summary.md`).
7. Merge the PR to finalize the changes.

## Creating the Proposal

### 1. Prepare the Summary File

Create a Markdown file (e.g., `proposal-summary.md`) with your proposal:

```markdown
# Proposal Title

## Overview
Brief description of what this proposal aims to achieve.

## Details
Detailed explanation of the proposal...

## Impact
Expected outcomes and benefits...

## Community Discussion
Link to forum discussion: [Forum Thread Title](https://forum.dfinity.org/t/...)
Discussion period: DD/MM/YYYY - DD/MM/YYYY
```

### 2. Optional: Prepare Motion Text

Create a separate file for detailed motion text (e.g., `motion-text.md`) if needed:

```markdown
This motion proposes to...

Technical details:
1. ...
2. ...

Implementation timeline:
- Phase 1: ...
- Phase 2: ...
```

### Step 3: Adjusting the Filename

After a successful submission, the governance canister will return a proposal ID.
Rename the motion proposal markdown file to include this ID in its filename (for example, updating the file from `proposal-summary.md` to `<proposalID> proposal-summary.md`).
Commit and push this change to the PR.

### Step 4: Merging the PR

Finally, merge the PR that includes the updated filename to have an official record of the submitted proposal in the repository, for future reference.

## Submission Examples

### Basic Submission
```bash
dre governance propose motion \
    --private-key-pem key.pem \
    --forum-post-link https://forum.dfinity.org/... \
    proposal-summary.md
```

### With Custom Title and Motion Text
```bash
dre governance propose motion \
    --private-key-pem key.pem \
    --forum-post-link https://forum.dfinity.org/... \
    --title "Custom Proposal Title" \
    --motion-text-file motion-text.md \
    proposal-summary.md
```

### Dry Run for Testing
```bash
dre governance propose motion \
    --forum-post-link https://forum.dfinity.org/... \
    --dry-run \
    proposal-summary.md
```

## Best Practices

1. **Community Engagement**
    - Start forum discussion early (at least 2 weeks before submission)
    - Actively engage with community feedback
    - Document changes made based on community input
    - Include forum discussion link in proposal

2. **Summary Format**
    - Start with a clear H1 heading for the title
    - Keep content under 30 KiB
    - Use Markdown formatting for readability
    - Include forum discussion link and timeline

3. **Motion Text**
    - Keep content under 100 KiB
    - Use clear, structured formatting
    - Include implementation details

4. **Testing**
    - Use `--dry-run` to verify proposal before submission
    - Verify forum integration with test post if using Discourse API

## Troubleshooting

### Common Issues

1. **File Size Limits**
    - Summary must be under 30 KiB
    - Motion text must be under 100 KiB

2. **Authentication**
    - Verify private key or HSM credentials
    - Check neuron ID and permissions

3. **Discourse Forum Integration**
    - Verify API key and permissions
    - Check forum URL format
    - Ensure post meets forum guidelines

4. **Forum Requirements**
    - Insufficient discussion period (less than 2 weeks)
    - Missing forum discussion link
    - Lack of community engagement

### Error Messages

- "Summary must be valid UTF-8": Check file encoding
- "Proposal submission failed": Verify neuron status and network connectivity
- No proposal ID returned: Check governance canister response

## Additional Resources

- [NNS Documentation](https://internetcomputer.org/docs/current/tokenomics/nns/nns-intro)
- [Internet Computer Forum](https://forum.dfinity.org/)
- [Markdown Guide](https://www.markdownguide.org/)
- [DRE CLI Documentation](https://dfinity.github.io/dre/)
