//! GCP Compute Engine API client

use std::path::PathBuf;

use reqwest::Client;
use slog::{Logger, debug, error, info};
use thiserror::Error;

use super::credentials::GcpCredentials;
use super::models::{
    AccessConfigRequest, AttachedDisk, CreateInstanceRequest, DiskInitializeParams,
    GcpInstance, GcpInstanceList, GcpOperation, NetworkInterfaceConfig, Tags,
};
use crate::models::{Vm, VmProvisionRequest, VmStatus};

const COMPUTE_API_BASE: &str = "https://compute.googleapis.com/compute/v1";

/// Errors from the GCP client
#[derive(Debug, Error)]
pub enum GcpClientError {
    #[error("GCP credentials not available")]
    NoCredentials,
    #[error("Failed to get access token: {0}")]
    TokenError(String),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

/// GCP Compute Engine client
pub struct GcpClient {
    credentials: GcpCredentials,
    http_client: Client,
    log: Logger,
}

impl GcpClient {
    /// Create a new GCP client
    pub async fn new(credentials_file: Option<PathBuf>, log: Logger) -> Self {
        let credentials = GcpCredentials::new(credentials_file, log.clone()).await;
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            credentials,
            http_client,
            log,
        }
    }

    /// Check if the client is available (has credentials)
    pub fn is_available(&self) -> bool {
        self.credentials.is_available()
    }

    /// Get authorization header
    async fn auth_header(&self) -> Result<String, GcpClientError> {
        if !self.credentials.is_available() {
            return Err(GcpClientError::NoCredentials);
        }
        let token = self
            .credentials
            .get_token()
            .await
            .map_err(|e| GcpClientError::TokenError(e.to_string()))?;
        Ok(format!("Bearer {}", token))
    }

    /// List instances in a project/zone
    pub async fn list_instances(
        &self,
        project: &str,
        zone: &str,
    ) -> Result<Vec<GcpInstance>, GcpClientError> {
        let url = format!(
            "{}/projects/{}/zones/{}/instances",
            COMPUTE_API_BASE, project, zone
        );

        debug!(self.log, "Listing instances"; "project" => project, "zone" => zone);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header().await?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            error!(self.log, "Failed to list instances"; "status" => status, "message" => &message);
            return Err(GcpClientError::ApiError { status, message });
        }

        let list: GcpInstanceList = response.json().await?;
        info!(self.log, "Found instances"; "count" => list.items.len());
        Ok(list.items)
    }

    /// List instances across multiple zones
    pub async fn list_instances_all_zones(
        &self,
        project: &str,
        zones: &[String],
    ) -> Result<Vec<GcpInstance>, GcpClientError> {
        let mut all_instances = Vec::new();

        for zone in zones {
            match self.list_instances(project, zone).await {
                Ok(instances) => all_instances.extend(instances),
                Err(e) => {
                    error!(self.log, "Failed to list instances in zone"; "zone" => zone, "error" => %e);
                    // Continue with other zones
                }
            }
        }

        Ok(all_instances)
    }

    /// Get a specific instance
    pub async fn get_instance(
        &self,
        project: &str,
        zone: &str,
        instance_name: &str,
    ) -> Result<GcpInstance, GcpClientError> {
        let url = format!(
            "{}/projects/{}/zones/{}/instances/{}",
            COMPUTE_API_BASE, project, zone, instance_name
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header().await?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(GcpClientError::ApiError { status, message });
        }

        Ok(response.json().await?)
    }

    /// Create a new instance
    pub async fn create_instance(
        &self,
        project: &str,
        zone: &str,
        request: &VmProvisionRequest,
    ) -> Result<GcpOperation, GcpClientError> {
        let url = format!(
            "{}/projects/{}/zones/{}/instances",
            COMPUTE_API_BASE, project, zone
        );

        let machine_type = format!(
            "zones/{}/machineTypes/{}",
            zone, request.machine_type
        );

        let disk_type = format!(
            "zones/{}/diskTypes/pd-ssd",
            zone
        );

        let create_request = CreateInstanceRequest {
            name: request.name.clone(),
            machine_type,
            disks: vec![AttachedDisk {
                boot: true,
                auto_delete: true,
                initialize_params: DiskInitializeParams {
                    disk_size_gb: request.disk_size_gb.to_string(),
                    source_image: "projects/ubuntu-os-cloud/global/images/family/ubuntu-2204-lts".to_string(),
                    disk_type,
                },
            }],
            network_interfaces: vec![NetworkInterfaceConfig {
                network: Some("global/networks/default".to_string()),
                subnetwork: None,
                access_configs: Some(vec![AccessConfigRequest {
                    config_type: "ONE_TO_ONE_NAT".to_string(),
                    name: "External NAT".to_string(),
                }]),
            }],
            labels: if request.labels.is_empty() {
                None
            } else {
                Some(request.labels.clone())
            },
            tags: if request.network_tags.is_empty() {
                None
            } else {
                Some(Tags {
                    items: request.network_tags.clone(),
                })
            },
        };

        info!(self.log, "Creating instance"; "name" => &request.name, "zone" => zone);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", self.auth_header().await?)
            .json(&create_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            error!(self.log, "Failed to create instance"; "status" => status, "message" => &message);
            return Err(GcpClientError::ApiError { status, message });
        }

        Ok(response.json().await?)
    }

    /// Delete an instance
    pub async fn delete_instance(
        &self,
        project: &str,
        zone: &str,
        instance_name: &str,
    ) -> Result<GcpOperation, GcpClientError> {
        let url = format!(
            "{}/projects/{}/zones/{}/instances/{}",
            COMPUTE_API_BASE, project, zone, instance_name
        );

        info!(self.log, "Deleting instance"; "name" => instance_name, "zone" => zone);

        let response = self
            .http_client
            .delete(&url)
            .header("Authorization", self.auth_header().await?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            error!(self.log, "Failed to delete instance"; "status" => status, "message" => &message);
            return Err(GcpClientError::ApiError { status, message });
        }

        Ok(response.json().await?)
    }

    /// Get operation status
    pub async fn get_operation(
        &self,
        project: &str,
        zone: &str,
        operation_name: &str,
    ) -> Result<GcpOperation, GcpClientError> {
        let url = format!(
            "{}/projects/{}/zones/{}/operations/{}",
            COMPUTE_API_BASE, project, zone, operation_name
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header().await?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(GcpClientError::ApiError { status, message });
        }

        Ok(response.json().await?)
    }

    /// Convert GCP instance to our VM model
    pub fn instance_to_vm(&self, instance: GcpInstance) -> Vm {
        Vm {
            id: instance.id.clone(),
            name: instance.name.clone(),
            zone: instance.zone_name(),
            machine_type: instance.machine_type_name(),
            status: VmStatus::from(instance.status.as_str()),
            internal_ip: instance.internal_ip(),
            external_ip: instance.external_ip(),
            ipv6_address: instance.ipv6_address(),
            created_at: instance.creation_timestamp,
            icp_node: None, // Will be populated by registry mapping
            labels: instance.labels,
        }
    }
}

impl Clone for GcpClient {
    fn clone(&self) -> Self {
        Self {
            credentials: self.credentials.clone(),
            http_client: self.http_client.clone(),
            log: self.log.clone(),
        }
    }
}
