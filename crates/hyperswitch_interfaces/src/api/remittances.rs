//! Remittances interface

use hyperswitch_domain_models::{
    router_flow_types::{Execute, RSync},
    router_request_types::RemittancesData,
    router_response_types::RemittancesResponseData,
};

use crate::api::{self, ConnectorCommon};

/// Trait for executing remittance transactions
pub trait RemittanceExecute:
    api::ConnectorIntegration<Execute, RemittancesData, RemittancesResponseData>
{
}

/// Trait for synchronizing remittance status
pub trait RemittanceSync: 
    api::ConnectorIntegration<RSync, RemittancesData, RemittancesResponseData> 
{
}

/// Main trait combining all remittance functionality
pub trait Remittance: ConnectorCommon + RemittanceExecute + RemittanceSync {}