# Setting Up Neuron Following for Subnet Upgrades

This guide describes how to set up neuron following for automated subnet upgrades on the Internet Computer. This enables a secure approval process for subnet upgrade proposals where multiple parties need to be involved.

## Overview

The neuron following setup ensures that automated subnet upgrade proposals require approval from both:
1. The automation system (primary proposer)
2. At least one authorized release team member

This provides a secure two-party approval system for subnet upgrades.

## Technical Details

- **Topic ID**: `12` (IC-OS Version Deployment)
- **Topic Reference**: Defined in the IC governance protobuf as `TOPIC_IC_OS_VERSION_DEPLOYMENT`

## Setting Up Neuron Following

To set up neuron following, you'll need:
- Access to dfx CLI
- Your neuron ID
- The ID of the neuron you want to follow
- HSM identity configured

### Command to Set Up Following

```bash
cd <path>/ic/rs/nns
# Export required variables
export NEURON_ID=<Your neuron ID>
export NEURON_TO_FOLLOW=<Neuron ID to follow>
export DFX_HSM_PIN="$(cat ~/.hsm-pin)"

# Set up following for your neuron
dfx --identity hsm canister --network mainnet call governance manage_neuron \
  "(record{ id = opt record{ id = ${NEURON_ID} : nat64 }; command = opt variant{Follow = record{ topic = 12 : int32; followees = vec{ record{ id = ${NEURON_TO_FOLLOW} : nat64 } } }}})"
```

### Verify Following Setup

You can verify your neuron following configuration with:

```bash
dfx --identity hsm canister --network mainnet call governance get_full_neuron "(${NEURON_ID}:nat64)"
```

The output will include a `followees` section showing all topics and neurons being followed.

## Notes

- Only configure following for Topic 12 (subnet replica version management)
- Ensure you're following the correct neuron based on your role
- Following relationships should be configured according to your organization's security policies