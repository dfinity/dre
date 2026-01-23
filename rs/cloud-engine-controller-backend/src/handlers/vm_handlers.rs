//! VM management handlers

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use slog::{error, info};

use crate::models::vm::{Vm, VmListResponse, VmProvisionRequest, VmProvisionResponse};
use crate::state::AppState;

/// Request with embedded auth token
#[derive(Debug, Deserialize)]
pub struct ListVmsRequest {
    /// Session token
    pub token: String,
}

/// Request with embedded auth token and provision data
#[derive(Debug, Deserialize)]
pub struct ProvisionVmRequest {
    /// Session token  
    pub token: String,
    /// VM name
    pub name: String,
    /// GCP zone
    pub zone: String,
    /// Machine type
    #[serde(default = "default_machine_type")]
    pub machine_type: String,
    /// Disk size in GB
    #[serde(default = "default_disk_size")]
    pub disk_size_gb: u64,
    /// Network tags
    #[serde(default)]
    pub network_tags: Vec<String>,
    /// Labels
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
    /// Configure as ICP node
    #[serde(default)]
    pub configure_as_icp_node: bool,
}

fn default_machine_type() -> String {
    "n2-standard-32".to_string()
}

fn default_disk_size() -> u64 {
    500
}

impl From<ProvisionVmRequest> for VmProvisionRequest {
    fn from(req: ProvisionVmRequest) -> Self {
        Self {
            name: req.name,
            zone: req.zone,
            machine_type: req.machine_type,
            disk_size_gb: req.disk_size_gb,
            network_tags: req.network_tags,
            labels: req.labels,
            configure_as_icp_node: req.configure_as_icp_node,
        }
    }
}

/// Request to delete VM
#[derive(Debug, Deserialize)]
pub struct DeleteVmRequest {
    /// Session token
    pub token: String,
    /// VM ID
    pub vm_id: String,
}

/// Helper to get session from token
fn get_session(state: &AppState, token: &str) -> Result<crate::auth::Session, (StatusCode, String)> {
    let session = state
        .sessions
        .get(token)
        .ok_or((StatusCode::UNAUTHORIZED, "Session not found".to_string()))?;
    
    if session.is_expired() {
        state.sessions.remove(token);
        return Err((StatusCode::UNAUTHORIZED, "Session expired".to_string()));
    }
    
    Ok(session.clone())
}

/// List VMs for the current user's GCP project
pub async fn list_vms(
    State(state): State<AppState>,
    Json(request): Json<ListVmsRequest>,
) -> Result<Json<VmListResponse>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    let user = state
        .get_user(&session.principal)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let gcp_account = user
        .gcp_account
        .ok_or((StatusCode::BAD_REQUEST, "GCP account not configured".to_string()))?;

    if !state.gcp_client.is_available() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "GCP client not available".to_string()));
    }

    // List VMs from all configured zones
    let instances = state
        .gcp_client
        .list_instances_all_zones(&gcp_account.project_id, &gcp_account.zones)
        .await
        .map_err(|e| {
            error!(state.log, "Failed to list VMs"; "error" => %e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list VMs: {}", e))
        })?;

    // Convert to our VM model and add ICP node mapping
    let vms: Vec<Vm> = instances
        .into_iter()
        .map(|instance| {
            let mut vm = state.gcp_client.instance_to_vm(instance);

            // Try to map to ICP node via IPv6 address
            if let Some(ref ipv6) = vm.ipv6_address {
                vm.icp_node = state.registry_manager.map_ip_to_node(ipv6);
            }
            // Fallback to internal IP
            if vm.icp_node.is_none() {
                if let Some(ref internal_ip) = vm.internal_ip {
                    vm.icp_node = state.registry_manager.map_ip_to_node(internal_ip);
                }
            }

            vm
        })
        .collect();

    let total = vms.len();
    Ok(Json(VmListResponse { vms, total }))
}

/// Provision a new VM
pub async fn provision_vm(
    State(state): State<AppState>,
    Json(request): Json<ProvisionVmRequest>,
) -> Result<Json<VmProvisionResponse>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    let user = state
        .get_user(&session.principal)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let gcp_account = user
        .gcp_account
        .ok_or((StatusCode::BAD_REQUEST, "GCP account not configured".to_string()))?;

    // Validate zone is in allowed list
    if !gcp_account.zones.contains(&request.zone) {
        return Err((StatusCode::BAD_REQUEST, format!(
            "Zone {} is not in your allowed zones: {:?}",
            request.zone, gcp_account.zones
        )));
    }

    if !state.gcp_client.is_available() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "GCP client not available".to_string()));
    }

    let provision_request: VmProvisionRequest = request.into();

    // Create the instance
    let operation = state
        .gcp_client
        .create_instance(&gcp_account.project_id, &provision_request.zone, &provision_request)
        .await
        .map_err(|e| {
            error!(state.log, "Failed to create VM"; "error" => %e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create VM: {}", e))
        })?;

    info!(state.log, "VM provisioning started"; 
        "name" => &provision_request.name, 
        "zone" => &provision_request.zone,
        "operation" => &operation.name
    );

    Ok(Json(VmProvisionResponse {
        operation_id: operation.name,
        vm_name: provision_request.name,
        zone: provision_request.zone,
        status: operation.status,
    }))
}

/// Delete a VM
pub async fn delete_vm(
    State(state): State<AppState>,
    Json(request): Json<DeleteVmRequest>,
) -> Result<Json<VmProvisionResponse>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    let user = state
        .get_user(&session.principal)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let gcp_account = user
        .gcp_account
        .ok_or((StatusCode::BAD_REQUEST, "GCP account not configured".to_string()))?;

    if !state.gcp_client.is_available() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "GCP client not available".to_string()));
    }

    // Parse vm_id which might be in format "zone/name" or just "name"
    let (zone, vm_name) = if request.vm_id.contains('/') {
        let parts: Vec<&str> = request.vm_id.splitn(2, '/').collect();
        (parts[0].to_string(), parts[1].to_string())
    } else {
        // Try to find the VM in all zones
        let mut found_zone = None;
        for zone in &gcp_account.zones {
            if let Ok(_instance) = state
                .gcp_client
                .get_instance(&gcp_account.project_id, zone, &request.vm_id)
                .await
            {
                found_zone = Some(zone.clone());
                break;
            }
        }
        match found_zone {
            Some(zone) => (zone, request.vm_id.clone()),
            None => return Err((StatusCode::NOT_FOUND, format!("VM {} not found", request.vm_id))),
        }
    };

    // Validate zone is in allowed list
    if !gcp_account.zones.contains(&zone) {
        return Err((StatusCode::BAD_REQUEST, format!(
            "Zone {} is not in your allowed zones",
            zone
        )));
    }

    // Delete the instance
    let operation = state
        .gcp_client
        .delete_instance(&gcp_account.project_id, &zone, &vm_name)
        .await
        .map_err(|e| {
            error!(state.log, "Failed to delete VM"; "error" => %e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete VM: {}", e))
        })?;

    info!(state.log, "VM deletion started"; 
        "name" => &vm_name, 
        "zone" => &zone,
        "operation" => &operation.name
    );

    Ok(Json(VmProvisionResponse {
        operation_id: operation.name,
        vm_name,
        zone,
        status: operation.status,
    }))
}
