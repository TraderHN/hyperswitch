//! Core remittances module
//! 
//! This module implements the core business logic for remittances,
//! orchestrating payments (funding from sender) and payouts (delivery to beneficiary)

pub mod flows;
pub mod helpers;
pub mod operations;
pub mod transformers;
pub mod validator;

use api_models::remittances::{
    RemittanceCreateRequest, RemittanceListRequest, RemittanceManualUpdateRequest,
    RemittancePayRequest, RemittanceQuoteRequest, RemittanceRetrieveRequest,
    RemittanceSyncRequest, RemittanceUpdateRequest,
};
use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, Encode},
    id_type,
};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
use uuid::Uuid;

use crate::{
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        payments as payments_core,
        payouts as payouts_core,
    },
    db::StorageInterface,
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, remittances as remittances_api},
        domain, storage,
    },
    utils::OptionExt,
};

// Re-export commonly used types
pub use operations::{
    RemittanceCancel, RemittanceCreate, RemittancePaymentConfirm, RemittanceQuote,
    RemittanceRetrieve, RemittanceSync, RemittanceUpdate,
};

/// Create a new remittance
#[instrument(skip_all)]
pub async fn create_remittance(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    req: RemittanceCreateRequest,
) -> RouterResponse<remittances_api::RemittanceResponse> {
    let operation = RemittanceCreate;
    
    flows::remittance_create_flow(
        &state,
        &req_state,
        &merchant_account,
        &profile,
        req,
        operation,
    )
    .await
}

/// Pay (fund) a remittance
#[instrument(skip_all)]
pub async fn pay_remittance(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    remittance_id: String,
    req: RemittancePayRequest,
) -> RouterResponse<remittances_api::RemittanceResponse> {
    let operation = RemittancePaymentConfirm;
    
    flows::remittance_pay_flow(
        &state,
        &req_state,
        &merchant_account,
        &profile,
        &remittance_id,
        req,
        operation,
    )
    .await
}

/// Retrieve a remittance
#[instrument(skip_all)]
pub async fn retrieve_remittance(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    remittance_id: String,
    req: RemittanceRetrieveRequest,
) -> RouterResponse<remittances_api::RemittanceResponse> {
    let operation = RemittanceRetrieve;
    
    flows::remittance_retrieve_flow(
        &state,
        &req_state,
        &merchant_account,
        &profile,
        &remittance_id,
        req,
        operation,
    )
    .await
}

/// Update a remittance
#[instrument(skip_all)]
pub async fn update_remittance(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    remittance_id: String,
    req: RemittanceUpdateRequest,
) -> RouterResponse<remittances_api::RemittanceResponse> {
    let operation = RemittanceUpdate;
    
    flows::remittance_update_flow(
        &state,
        &req_state,
        &merchant_account,
        &profile,
        &remittance_id,
        req,
        operation,
    )
    .await
}

/// Cancel a remittance
#[instrument(skip_all)]
pub async fn cancel_remittance(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    remittance_id: String,
) -> RouterResponse<remittances_api::RemittanceResponse> {
    let operation = RemittanceCancel;
    
    flows::remittance_cancel_flow(
        &state,
        &req_state,
        &merchant_account,
        &profile,
        &remittance_id,
        operation,
    )
    .await
}

/// List remittances
#[instrument(skip_all)]
pub async fn list_remittances(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    req: RemittanceListRequest,
) -> RouterResponse<remittances_api::RemittanceListResponse> {
    validator::validate_list_request(&req)?;
    
    let db = &state.store;
    let merchant_id = &merchant_account.merchant_id;
    
    let limit = req.limit.unwrap_or(10).min(100);
    let offset = req.offset.unwrap_or(0);
    
    // Get remittances with filters
    let remittances = db
        .filter_remittances_by_constraints(
            merchant_id,
            &req,
            limit as i64,
            offset as i64,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Get total count
    let total_count = db
        .get_remittances_count(merchant_id, &req)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Transform to API response
    let mut response_items = Vec::with_capacity(remittances.len());
    for remittance in remittances {
        let item = transformers::foreign_to_api_remittance(
            &state,
            remittance,
            &merchant_account,
            &profile,
        )
        .await?;
        response_items.push(item);
    }
    
    Ok(services::ApplicationResponse::Json(
        remittances_api::RemittanceListResponse {
            count: response_items.len(),
            total_count,
            data: response_items,
        },
    ))
}

/// Get exchange rate quote
#[instrument(skip_all)]
pub async fn get_remittance_quote(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    req: RemittanceQuoteRequest,
) -> RouterResponse<remittances_api::RemittanceQuoteResponse> {
    let operation = RemittanceQuote;
    
    flows::remittance_quote_flow(
        &state,
        &req_state,
        &merchant_account,
        &profile,
        req,
        operation,
    )
    .await
}

/// Sync remittance status with connector
#[instrument(skip_all)]
pub async fn sync_remittance(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    remittance_id: String,
) -> RouterResponse<remittances_api::RemittanceResponse> {
    let operation = RemittanceSync;
    
    flows::remittance_sync_flow(
        &state,
        &req_state,
        &merchant_account,
        &profile,
        &remittance_id,
        operation,
    )
    .await
}

/// Sync multiple remittances
#[instrument(skip_all)]
pub async fn sync_remittances_batch(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    req: RemittanceSyncRequest,
) -> RouterResponse<remittances_api::RemittanceSyncResponse> {
    validator::validate_sync_request(&req)?;
    
    let merchant_id = &merchant_account.merchant_id;
    let db = &state.store;
    
    // Get remittances to sync based on time range
    let filter_req = RemittanceListRequest {
        time_range: req.time_range,
        status: Some(vec![
            api_models::remittances::RemittanceStatus::PaymentInitiated,
            api_models::remittances::RemittanceStatus::PaymentProcessed,
            api_models::remittances::RemittanceStatus::PayoutInitiated,
        ]),
        ..Default::default()
    };
    
    let remittances_to_sync = db
        .filter_remittances_by_constraints(
            merchant_id,
            &filter_req,
            100, // Max batch size
            0,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    let mut sync_results = Vec::with_capacity(remittances_to_sync.len());
    
    for remittance in remittances_to_sync {
        let remittance_id = remittance.id.to_string();
        let previous_status = remittance.status.clone();
        
        // Sync individual remittance
        match flows::sync_single_remittance(
            &state,
            &req_state,
            &merchant_account,
            remittance,
            req.force_sync.unwrap_or(false),
        )
        .await
        {
            Ok(updated_remittance) => {
                sync_results.push(remittances_api::RemittanceSyncResult {
                    remittance_id,
                    previous_status: previous_status.parse().unwrap_or_default(),
                    current_status: updated_remittance.status.parse().unwrap_or_default(),
                    synced_at: common_utils::date_time::now(),
                    payment_updated: updated_remittance.payment_updated,
                    payout_updated: updated_remittance.payout_updated,
                });
            }
            Err(error) => {
                router_env::logger::error!(
                    ?error,
                    "Failed to sync remittance {}", remittance_id
                );
                // Continue with next remittance
            }
        }
    }
    
    Ok(services::ApplicationResponse::Json(
        remittances_api::RemittanceSyncResponse {
            merchant_id: merchant_id.clone(),
            synced_count: sync_results.len(),
            results: sync_results,
        },
    ))
}

/// Manual update by admin
#[instrument(skip_all)]
pub async fn manual_update_remittance(
    state: SessionState,
    req: RemittanceManualUpdateRequest,
) -> RouterResponse<remittances_api::RemittanceResponse> {
    // Verify admin permissions
    // This would typically be handled by middleware, but adding check here for safety
    
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    // Get and update remittance
    let remittance_uuid = Uuid::parse_str(&req.remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "remittance_id".to_string(),
            expected_format: "valid UUID".to_string(),
        })?;
    
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            &req.merchant_id,
            key_manager_state,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Build update
    let update = storage::RemittanceUpdate::ManualUpdate {
        status: Some(req.status.to_string()),
        failure_reason: req.error_reason.or(req.error_message),
        updated_at: common_utils::date_time::now(),
    };
    
    let updated_remittance = db
        .update_remittance(
            remittance,
            update,
            key_manager_state,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Transform to API response
    transformers::storage_to_api_remittance(updated_remittance)
}

/// Handle webhook from connector
#[instrument(skip_all)]
pub async fn handle_remittance_webhook(
    state: SessionState,
    req_state: ReqState,
    merchant_id: String,
    connector_name: String,
    body: serde_json::Value,
) -> RouterResponse<serde_json::Value> {
    router_env::logger::info!(
        ?connector_name,
        ?body,
        "Received remittance webhook"
    );
    
    // Parse webhook based on connector
    let webhook_data = helpers::parse_connector_webhook(&connector_name, &body)?;
    
    // Find remittance by connector reference
    let db = &state.store;
    let remittance = helpers::find_remittance_by_connector_reference(
        db,
        &merchant_id,
        &webhook_data.reference_id,
    )
    .await?;
    
    // Update remittance status based on webhook
    flows::process_remittance_webhook(
        &state,
        &req_state,
        remittance,
        webhook_data,
    )
    .await?;
    
    Ok(services::ApplicationResponse::Json(
        serde_json::json!({
            "status": "ok"
        })
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_remittance_create_flow() {
        // Add comprehensive tests
    }
    
    #[tokio::test]
    async fn test_remittance_payment_flow() {
        // Add comprehensive tests
    }
}