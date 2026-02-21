//! Integration tests for the Cloud Engine Controller Backend

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[cfg(test)]
mod auth_tests {
    use cloud_engine_controller_backend::auth::ii_delegation::{
        Delegation, DelegationChain, SignedDelegation, verify_delegation,
    };
    use super::*;

    #[test]
    fn test_empty_delegation_chain_rejected() {
        let chain = DelegationChain {
            public_key: "test".to_string(),
            delegations: vec![],
        };
        let result = verify_delegation(&chain, "session_key");
        assert!(result.is_err());
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
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_delegation_accepted() {
        let future_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
            + 3600_000_000_000; // 1 hour from now

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

#[cfg(test)]
mod session_tests {
    use cloud_engine_controller_backend::auth::middleware::Session;
    use chrono::{Duration, Utc};

    #[test]
    fn test_session_expiration() {
        let expired_session = Session {
            id: "test".to_string(),
            principal: "test-principal".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() - Duration::hours(1),
        };
        assert!(expired_session.is_expired());

        let valid_session = Session {
            id: "test".to_string(),
            principal: "test-principal".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        assert!(!valid_session.is_expired());
    }
}

#[cfg(test)]
mod model_tests {
    use cloud_engine_controller_backend::models::vm::VmStatus;
    use cloud_engine_controller_backend::models::user::User;

    #[test]
    fn test_vm_status_from_string() {
        assert_eq!(VmStatus::from("RUNNING"), VmStatus::Running);
        assert_eq!(VmStatus::from("STOPPED"), VmStatus::Stopped);
        assert_eq!(VmStatus::from("running"), VmStatus::Running);
        assert_eq!(VmStatus::from("unknown_status"), VmStatus::Unknown);
    }

    #[test]
    fn test_user_creation() {
        let user = User::new("test-principal".to_string());
        assert_eq!(user.principal, "test-principal");
        assert!(user.gcp_account.is_none());
        assert!(user.node_operator.is_none());
    }
}

#[cfg(test)]
mod node_mapping_tests {
    use cloud_engine_controller_backend::registry::node_mapping::NodeMapper;

    #[test]
    fn test_ipv4_mapping() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("192.168.1.1".to_string(), "node-1".to_string());
        
        assert_eq!(mapper.get_node_id("192.168.1.1"), Some("node-1".to_string()));
        assert_eq!(mapper.get_node_id("192.168.1.2"), None);
        assert!(mapper.is_icp_node("192.168.1.1"));
        assert!(!mapper.is_icp_node("192.168.1.2"));
    }

    #[test]
    fn test_ipv6_mapping() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("2001:db8::1".to_string(), "node-1".to_string());
        
        assert_eq!(mapper.get_node_id("2001:db8::1"), Some("node-1".to_string()));
        // Test with brackets (URL format)
        assert_eq!(mapper.get_node_id("[2001:db8::1]"), Some("node-1".to_string()));
    }

    #[test]
    fn test_reverse_mapping() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("192.168.1.1".to_string(), "node-1".to_string());
        mapper.add_mapping("2001:db8::1".to_string(), "node-1".to_string());
        
        let ips = mapper.get_ips_for_node("node-1");
        assert_eq!(ips.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("192.168.1.1".to_string(), "node-1".to_string());
        
        assert!(mapper.is_icp_node("192.168.1.1"));
        
        mapper.clear();
        
        assert!(!mapper.is_icp_node("192.168.1.1"));
    }

    #[test]
    fn test_stats() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("192.168.1.1".to_string(), "node-1".to_string());
        mapper.add_mapping("2001:db8::1".to_string(), "node-2".to_string());
        
        let stats = mapper.stats();
        assert_eq!(stats.ipv4_count, 1);
        assert_eq!(stats.ipv6_count, 1);
        assert_eq!(stats.node_count, 2);
    }
}

#[cfg(test)]
mod gcp_model_tests {
    use cloud_engine_controller_backend::gcp::models::GcpInstance;
    use std::collections::HashMap;

    #[test]
    fn test_gcp_instance_zone_extraction() {
        let instance = GcpInstance {
            id: "123".to_string(),
            name: "test-instance".to_string(),
            zone: "https://www.googleapis.com/compute/v1/projects/my-project/zones/us-central1-a".to_string(),
            machine_type: "https://www.googleapis.com/compute/v1/projects/my-project/zones/us-central1-a/machineTypes/n2-standard-8".to_string(),
            status: "RUNNING".to_string(),
            creation_timestamp: None,
            network_interfaces: vec![],
            labels: HashMap::new(),
            description: None,
        };

        assert_eq!(instance.zone_name(), "us-central1-a");
        assert_eq!(instance.machine_type_name(), "n2-standard-8");
    }
}

#[cfg(test)]
mod subnet_model_tests {
    use cloud_engine_controller_backend::models::subnet::ProposalStatus;

    #[test]
    fn test_proposal_status_serialization() {
        let status = ProposalStatus::Draft;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"draft\"");

        let status = ProposalStatus::Executed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"executed\"");
    }
}
