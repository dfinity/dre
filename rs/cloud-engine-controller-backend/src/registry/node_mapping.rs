//! Node mapping utilities for matching VMs to ICP nodes

use std::collections::HashMap;
use std::net::IpAddr;

/// Maps IP addresses to ICP node IDs
#[derive(Debug, Default)]
pub struct NodeMapper {
    /// IPv4 address to node ID mapping
    ipv4_to_node: HashMap<String, String>,
    /// IPv6 address to node ID mapping
    ipv6_to_node: HashMap<String, String>,
    /// Node ID to IP addresses
    node_to_ips: HashMap<String, Vec<String>>,
}

impl NodeMapper {
    /// Create a new empty node mapper
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        self.ipv4_to_node.clear();
        self.ipv6_to_node.clear();
        self.node_to_ips.clear();
    }

    /// Add a mapping from IP to node
    pub fn add_mapping(&mut self, ip: String, node_id: String) {
        // Normalize the IP address
        let normalized = Self::normalize_ip(&ip);
        
        // Determine if IPv4 or IPv6
        if let Ok(addr) = normalized.parse::<IpAddr>() {
            match addr {
                IpAddr::V4(_) => {
                    self.ipv4_to_node.insert(normalized.clone(), node_id.clone());
                }
                IpAddr::V6(_) => {
                    self.ipv6_to_node.insert(normalized.clone(), node_id.clone());
                }
            }
        } else {
            // If we can't parse, try both maps (might be a hostname or partial)
            self.ipv6_to_node.insert(normalized.clone(), node_id.clone());
        }

        // Add reverse mapping
        self.node_to_ips
            .entry(node_id)
            .or_default()
            .push(normalized);
    }

    /// Get node ID for an IP address
    pub fn get_node_id(&self, ip: &str) -> Option<String> {
        let normalized = Self::normalize_ip(ip);
        
        self.ipv4_to_node
            .get(&normalized)
            .or_else(|| self.ipv6_to_node.get(&normalized))
            .cloned()
    }

    /// Get IP addresses for a node
    pub fn get_ips_for_node(&self, node_id: &str) -> Vec<String> {
        self.node_to_ips
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if an IP is mapped to any node
    pub fn is_icp_node(&self, ip: &str) -> bool {
        self.get_node_id(ip).is_some()
    }

    /// Normalize an IP address string
    fn normalize_ip(ip: &str) -> String {
        // Remove brackets if present (IPv6 in URLs)
        let trimmed = ip.trim_matches(|c| c == '[' || c == ']');
        
        // Parse and reformat to canonical form
        if let Ok(addr) = trimmed.parse::<IpAddr>() {
            addr.to_string()
        } else {
            trimmed.to_lowercase()
        }
    }

    /// Get statistics about the mapper
    pub fn stats(&self) -> NodeMapperStats {
        NodeMapperStats {
            ipv4_count: self.ipv4_to_node.len(),
            ipv6_count: self.ipv6_to_node.len(),
            node_count: self.node_to_ips.len(),
        }
    }
}

/// Statistics about the node mapper
#[derive(Debug, Clone)]
pub struct NodeMapperStats {
    pub ipv4_count: usize,
    pub ipv6_count: usize,
    pub node_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_lookup_ipv4() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("192.168.1.1".to_string(), "node-1".to_string());
        
        assert_eq!(mapper.get_node_id("192.168.1.1"), Some("node-1".to_string()));
        assert_eq!(mapper.get_node_id("192.168.1.2"), None);
    }

    #[test]
    fn test_add_and_lookup_ipv6() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("2001:db8::1".to_string(), "node-1".to_string());
        
        assert_eq!(mapper.get_node_id("2001:db8::1"), Some("node-1".to_string()));
        assert_eq!(mapper.get_node_id("[2001:db8::1]"), Some("node-1".to_string()));
    }

    #[test]
    fn test_get_ips_for_node() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("192.168.1.1".to_string(), "node-1".to_string());
        mapper.add_mapping("2001:db8::1".to_string(), "node-1".to_string());
        
        let ips = mapper.get_ips_for_node("node-1");
        assert_eq!(ips.len(), 2);
        assert!(ips.contains(&"192.168.1.1".to_string()));
        assert!(ips.contains(&"2001:db8::1".to_string()));
    }

    #[test]
    fn test_clear() {
        let mut mapper = NodeMapper::new();
        mapper.add_mapping("192.168.1.1".to_string(), "node-1".to_string());
        
        assert!(mapper.is_icp_node("192.168.1.1"));
        
        mapper.clear();
        
        assert!(!mapper.is_icp_node("192.168.1.1"));
    }
}
