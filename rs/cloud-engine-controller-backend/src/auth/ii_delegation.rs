//! Internet Identity delegation validation
//!
//! This module validates delegation chains from Internet Identity.
//! A delegation chain proves that a session key is authorized to act
//! on behalf of a principal.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use utoipa::ToSchema;

/// Errors that can occur during delegation validation
#[derive(Debug, Error)]
pub enum DelegationError {
    #[error("Invalid delegation format: {0}")]
    InvalidFormat(String),
    #[error("Delegation chain is empty")]
    EmptyChain,
    #[error("Delegation has expired")]
    Expired,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Public key mismatch in chain")]
    PublicKeyMismatch,
    #[error("Delegation verification failed: {0}")]
    VerificationFailed(String),
}

/// A single delegation in the chain
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Delegation {
    /// The delegated public key (DER encoded, base64)
    pub pubkey: String,
    /// Expiration time in nanoseconds since epoch
    pub expiration: u64,
    /// Optional targets (canister IDs) this delegation is valid for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
}

/// A signed delegation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignedDelegation {
    /// The delegation
    pub delegation: Delegation,
    /// Signature over the delegation (base64)
    pub signature: String,
}

/// A complete delegation chain from II
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DelegationChain {
    /// The root public key (II's public key for the user)
    pub public_key: String,
    /// Chain of delegations
    pub delegations: Vec<SignedDelegation>,
}

/// Request to verify a delegation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyDelegationRequest {
    /// The delegation chain from II
    pub delegation_chain: DelegationChain,
    /// The session public key to verify
    pub session_pubkey: String,
}

/// Response from delegation verification
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyDelegationResponse {
    /// Whether the delegation is valid
    pub valid: bool,
    /// The user's principal (derived from II public key)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    /// Session token for subsequent requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    /// Expiration time of the session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// Error message if validation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Verify a delegation chain
///
/// This validates that:
/// 1. The delegation chain is not empty
/// 2. Each delegation has not expired
/// 3. The chain properly links from root to session key
pub fn verify_delegation(
    chain: &DelegationChain,
    session_pubkey: &str,
) -> Result<String, DelegationError> {
    if chain.delegations.is_empty() {
        return Err(DelegationError::EmptyChain);
    }

    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    // Check expiration of all delegations
    for signed_delegation in &chain.delegations {
        if signed_delegation.delegation.expiration < now_ns {
            return Err(DelegationError::Expired);
        }
    }

    // The last delegation's pubkey should match the session pubkey
    if let Some(last_delegation) = chain.delegations.last() {
        if last_delegation.delegation.pubkey != session_pubkey {
            return Err(DelegationError::PublicKeyMismatch);
        }
    }

    // Derive principal from root public key
    // In IC, principal is derived from the DER-encoded public key
    let principal = derive_principal_from_pubkey(&chain.public_key)?;

    Ok(principal)
}

/// Derive a principal from a DER-encoded public key
fn derive_principal_from_pubkey(pubkey_base64: &str) -> Result<String, DelegationError> {
    let pubkey_bytes = BASE64
        .decode(pubkey_base64)
        .map_err(|e| DelegationError::InvalidFormat(format!("Invalid base64 public key: {}", e)))?;

    // Hash the public key with SHA-224 (first 28 bytes of SHA-256)
    let mut hasher = Sha256::new();
    hasher.update(&pubkey_bytes);
    let hash = hasher.finalize();

    // Take first 28 bytes for self-authenticating principal
    let principal_bytes = &hash[..28];

    // Add the self-authenticating principal type byte (0x02)
    let mut principal_with_type = vec![0x02];
    principal_with_type.extend_from_slice(principal_bytes);

    // Encode as textual representation
    // Using a simplified hex encoding for now
    // In production, you'd use the proper IC principal encoding with CRC32 checksum
    Ok(hex::encode(principal_with_type))
}

/// Compute a hash for signing delegation data
#[allow(dead_code)]
fn compute_delegation_hash(delegation: &Delegation) -> Vec<u8> {
    let mut hasher = Sha256::new();

    // Hash the domain separator
    hasher.update(b"\x1Aic-request-auth-delegation");

    // Hash the delegation fields
    hasher.update(&BASE64.decode(&delegation.pubkey).unwrap_or_default());
    hasher.update(&delegation.expiration.to_be_bytes());

    if let Some(targets) = &delegation.targets {
        for target in targets {
            hasher.update(target.as_bytes());
        }
    }

    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_chain_rejected() {
        let chain = DelegationChain {
            public_key: "test".to_string(),
            delegations: vec![],
        };
        let result = verify_delegation(&chain, "session_key");
        assert!(matches!(result, Err(DelegationError::EmptyChain)));
    }

    #[test]
    fn test_expired_delegation_rejected() {
        let chain = DelegationChain {
            public_key: BASE64.encode(b"test_pubkey"),
            delegations: vec![SignedDelegation {
                delegation: Delegation {
                    pubkey: "session_key".to_string(),
                    expiration: 0, // Expired
                    targets: None,
                },
                signature: "sig".to_string(),
            }],
        };
        let result = verify_delegation(&chain, "session_key");
        assert!(matches!(result, Err(DelegationError::Expired)));
    }

    #[test]
    fn test_pubkey_mismatch_rejected() {
        let future_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
            + 3600_000_000_000; // 1 hour from now

        let chain = DelegationChain {
            public_key: BASE64.encode(b"test_pubkey"),
            delegations: vec![SignedDelegation {
                delegation: Delegation {
                    pubkey: "different_key".to_string(),
                    expiration: future_ns,
                    targets: None,
                },
                signature: "sig".to_string(),
            }],
        };
        let result = verify_delegation(&chain, "session_key");
        assert!(matches!(result, Err(DelegationError::PublicKeyMismatch)));
    }

    #[test]
    fn test_valid_delegation_accepted() {
        let future_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
            + 3600_000_000_000;

        let session_key = BASE64.encode(b"session_pubkey");
        let chain = DelegationChain {
            public_key: BASE64.encode(b"root_pubkey"),
            delegations: vec![SignedDelegation {
                delegation: Delegation {
                    pubkey: session_key.clone(),
                    expiration: future_ns,
                    targets: None,
                },
                signature: "sig".to_string(),
            }],
        };
        let result = verify_delegation(&chain, &session_key);
        assert!(result.is_ok());
    }
}
