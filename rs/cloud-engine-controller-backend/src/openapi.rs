//! OpenAPI/Swagger configuration

use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::auth::ii_delegation::{
    Delegation, DelegationChain, SignedDelegation, VerifyDelegationRequest,
    VerifyDelegationResponse,
};
use crate::auth::middleware::SessionInfo;
use crate::handlers::node_handlers::{NodeInfo, NodeListResponse};
use crate::models::subnet::{
    ProposalStatus, SubnetInfo, SubnetListResponse, SubnetProposal,
    SubnetProposalResponse,
};
use crate::models::user::{
    GcpAccount, NodeOperatorInfo, UserProfile,
};
use crate::models::vm::{
    IcpNodeMapping, Vm, VmListResponse, VmProvisionRequest, VmProvisionResponse, VmStatus,
};

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Cloud Engine Controller Backend",
        version = "1.0.0",
        description = "Backend API for managing GCP VMs and ICP node associations. All authenticated endpoints require a 'token' field in the request body.",
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
            // Auth schemas
            Delegation,
            SignedDelegation,
            DelegationChain,
            VerifyDelegationRequest,
            VerifyDelegationResponse,
            SessionInfo,
            // User schemas
            GcpAccount,
            NodeOperatorInfo,
            UserProfile,
            // VM schemas
            VmStatus,
            IcpNodeMapping,
            Vm,
            VmProvisionRequest,
            VmProvisionResponse,
            VmListResponse,
            // Node schemas
            NodeInfo,
            NodeListResponse,
            // Subnet schemas
            ProposalStatus,
            SubnetInfo,
            SubnetProposal,
            SubnetProposalResponse,
            SubnetListResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Authentication", description = "Internet Identity authentication endpoints"),
        (name = "User", description = "User profile management"),
        (name = "VMs", description = "GCP VM management"),
        (name = "Nodes", description = "ICP node information"),
        (name = "Subnets", description = "Subnet management via NNS proposals")
    )
)]
pub struct ApiDoc;

/// Security scheme modifier
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}
