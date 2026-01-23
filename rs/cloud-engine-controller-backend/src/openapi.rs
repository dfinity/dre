//! OpenAPI/Swagger configuration

use utoipa::OpenApi;

use crate::config::{AppConfig, GcpConfig, NodeOperatorConfig};
use crate::handlers::node_handlers::{GetNodeRequest, NodeInfo, NodeListResponse};
use crate::handlers::subnet_handlers::{CreateSubnetProposalRequest, DeleteSubnetRequest, SubnetUpgradeResponse};
use crate::handlers::vm_handlers::{DeleteVmRequest, ProvisionVmRequestBody};
use crate::models::subnet::{ProposalStatus, SubnetInfo, SubnetListResponse, SubnetProposal, SubnetProposalResponse, SubnetUpgradeRequest};
use crate::models::vm::{IcpNodeMapping, Vm, VmListResponse, VmProvisionRequest, VmProvisionResponse, VmStatus};

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        // Config endpoints
        crate::handlers::vm_handlers::get_config,
        // VM endpoints
        crate::handlers::vm_handlers::list_vms,
        crate::handlers::vm_handlers::provision_vm,
        crate::handlers::vm_handlers::delete_vm,
        // Node endpoints
        crate::handlers::node_handlers::list_nodes,
        crate::handlers::node_handlers::get_node,
        // Subnet endpoints
        crate::handlers::subnet_handlers::list_subnets,
        crate::handlers::subnet_handlers::create_subnet_proposal,
        crate::handlers::subnet_handlers::delete_subnet_proposal,
        crate::handlers::subnet_handlers::upgrade_subnet,
    ),
    info(
        title = "Cloud Engine Controller Backend",
        version = "1.0.0",
        description = "Backend API for managing GCP VMs and ICP node associations. Configuration (GCP project, zones, node operator) is provided via a config file.",
        contact(
            name = "DRE Team",
            url = "https://github.com/dfinity/dre"
        ),
        license(
            name = "Apache-2.0",
            url = "https://www.apache.org/licenses/LICENSE-2.0"
        )
    ),
    components(
        schemas(
            // Config schemas
            AppConfig,
            GcpConfig,
            NodeOperatorConfig,
            // VM schemas
            VmStatus,
            IcpNodeMapping,
            Vm,
            VmProvisionRequest,
            VmProvisionResponse,
            VmListResponse,
            ProvisionVmRequestBody,
            DeleteVmRequest,
            // Node schemas
            NodeInfo,
            NodeListResponse,
            GetNodeRequest,
            // Subnet schemas
            ProposalStatus,
            SubnetInfo,
            SubnetProposal,
            SubnetProposalResponse,
            SubnetListResponse,
            CreateSubnetProposalRequest,
            DeleteSubnetRequest,
            SubnetUpgradeRequest,
            SubnetUpgradeResponse,
        )
    ),
    tags(
        (name = "Config", description = "Configuration endpoints"),
        (name = "VMs", description = "GCP VM management"),
        (name = "Nodes", description = "ICP node information"),
        (name = "Subnets", description = "Subnet management via NNS proposals")
    )
)]
pub struct ApiDoc;
