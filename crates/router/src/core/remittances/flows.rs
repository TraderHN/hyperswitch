//! Remittance flow orchestration

use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        payments as payments_core,
        payouts as payouts_core,
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, payments as payment_types, payouts as payout_types},
        domain, storage,
    },
};

use super::{helpers, operations::RemittanceOperation, transformers, validator};

/// Main flow for creating a remittance
#[instrument(skip_all)]
pub async fn remittance_create_flow<Op>(
    state: &SessionState,
    req_state: &ReqState,
    merchant_account: &domain::MerchantAccount,
    profile: &domain::Profile,
    req: api::remittances::RemittanceCreateRequest,
    operation: Op,
) -> RouterResponse<api::remittances::RemittanceResponse>
where
    Op: RemittanceOperation
        Request = api::remittances::RemittanceCreateRequest,
        Response = storage::Remittance,
    >,
{
    // Validate request
    operation
        .validate_request(state, merchant_account, profile, &req)
        .await?;
    
    // Execute operation
    let remittance = operation
        .execute(state, req_state, merchant_account, profile, req.clone())
        .await?;
    
    // Transform to API response
    let response = transformers::foreign_to_api_remittance(
        state,
        remittance,
        merchant_account,
        profile,
    )
    .await?;
    
    // If auto_process is enabled, initiate payment immediately
    if req.auto_process.unwrap_or(false) {
        tokio::spawn(async move {
            if let Err(e) = initiate_auto_payment(
                state.clone(),
                req_state.clone(),
                response.remittance_id.clone(),
            )
            .await
            {
                router_env::logger::error!(
                    ?e,
                    "Failed to auto-process payment for remittance {}",
                    response.remittance_id
                );
            }
        });
    }
    
    Ok(services::ApplicationResponse::Json(response))
}

/// Flow for paying (funding) a remittance
#[instrument(skip_all)]
pub async fn remittance_pay_flow<Op>(
    state: &SessionState,
    req_state: &ReqState,
    merchant_account: &domain::MerchantAccount,
    profile: &domain::Profile,
    remittance_id: &str,
    req: api::remittances::RemittancePayRequest,
    _operation: Op,
) -> RouterResponse<api::remittances::RemittanceResponse>
where
    Op: RemittanceOperation,
{
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    // Get remittance
    let remittance_uuid = uuid::Uuid::parse_str(remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "remittance_id".to_string(),
            expected_format: "valid UUID".to_string(),
        })?;
    
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            &merchant_account.merchant_id,
            key_manager_state,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Validate remittance can be paid
    validator::validate_remittance_payable(&remittance)?;
    
    // Update status to PaymentInitiated
    let status_update = storage::RemittanceUpdate::StatusUpdate {
        status: storage::RemittanceStatus::PaymentInitiated.to_string(),
        updated_at: common_utils::date_time::now(),
    };
    
    let remittance = db
        .update_remittance(remittance, status_update, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Create payment using existing payment infrastructure
    let payment_response = create_remittance_payment(
        state,
        req_state,
        merchant_account,
        profile,
        &remittance,
        req,
    )
    .await?;
    
    // Update remittance payment record
    let payment_update = storage::RemittancePaymentUpdate {
        payment_id: Some(payment_response.payment_id.to_string()),
        status: Some(payment_response.status.to_string()),
        auth_type: payment_response.authentication_type.map(|t| t.to_string()),
        updated_at: Some(common_utils::date_time::now()),
        ..Default::default()
    };
    
    db.update_remittance_payment_by_remittance_id(
        &remittance_uuid,
        payment_update,
        key_manager_state,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
    
    // If payment successful, initiate payout
    if payment_response.status == api_models::enums::IntentStatus::Succeeded {
        tokio::spawn(async move {
            if let Err(e) = initiate_remittance_payout(
                state.clone(),
                req_state.clone(),
                remittance.id,
            )
            .await
            {
                router_env::logger::error!(
                    ?e,
                    "Failed to initiate payout for remittance {}",
                    remittance.id
                );
            }
        });
    }
    
    // Get updated remittance and transform to response
    let updated_remittance = db
        .find_remittance_by_id(&remittance_uuid, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    let response = transformers::foreign_to_api_remittance(
        state,
        updated_remittance,
        merchant_account,
        profile,
    )
    .await?;
    
    Ok(services::ApplicationResponse::Json(response))
}

/// Create payment for remittance funding
async fn create_remittance_payment(
    state: &SessionState,
    req_state: &ReqState,
    merchant_account: &domain::MerchantAccount,
    profile: &domain::Profile,
    remittance: &storage::Remittance,
    pay_request: api::remittances::RemittancePayRequest,
) -> CustomResult<payment_types::PaymentsResponse, errors::ApiErrorResponse> {
    // Transform remittance pay request to payment request
    let payment_request = transformers::remittance_to_payment_request(
        remittance,
        pay_request,
    )?;
    
    // Use existing payment core to process
    let payment_response = payments_core::payments_core(
        state.clone(),
        req_state.clone(),
        merchant_account.clone(),
        profile.clone(),
        payments_core::operations::PaymentCreate,
        payment_request,
        services::AuthFlow::Merchant,
        payments_core::CallConnectorAction::Trigger,
        None,
    )
    .await?;
    
    Ok(payment_response)
}

/// Initiate payout to beneficiary
async fn initiate_remittance_payout(
    state: SessionState,
    req_state: ReqState,
    remittance_id: uuid::Uuid,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    // Get remittance
    let remittance = db
        .find_remittance_by_id(&remittance_id, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Get merchant and profile
    let merchant_account = db
        .find_merchant_account_by_merchant_id(&remittance.merchant_id, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    
    let profile = db
        .find_business_profile_by_profile_id(&remittance.profile_id, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound)?;
    
    // Update status to PayoutInitiated
    let status_update = storage::RemittanceUpdate::StatusUpdate {
        status: storage::RemittanceStatus::PayoutInitiated.to_string(),
        updated_at: common_utils::date_time::now(),
    };
    
    let remittance = db
        .update_remittance(remittance, status_update, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Create payout request
    let payout_request = transformers::remittance_to_payout_request(&remittance)?;
    
    // Use existing payout core
    let payout_response = payouts_core::payouts_create_core(
        state.clone(),
        req_state.clone(),
        merchant_account,
        profile,
        payout_request,
    )
    .await?;
    
    // Update remittance payout record
    let payout_update = storage::RemittancePayoutUpdate {
        payout_id: Some(payout_response.payout_id.to_string()),
        status: Some(payout_response.status.to_string()),
        updated_at: Some(common_utils::date_time::now()),
        ..Default::default()
    };
    
    db.update_remittance_payout_by_remittance_id(
        &remittance_id,
        payout_update,
        key_manager_state,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
    
    // Update remittance status based on payout status
    let final_status = match payout_response.status {
        api_models::enums::PayoutStatus::Success => storage::RemittanceStatus::Completed,
        api_models::enums::PayoutStatus::Failed => storage::RemittanceStatus::Failed,
        _ => return Ok(()), // Still processing
    };
    
    let final_update = storage::RemittanceUpdate::StatusUpdate {
        status: final_status.to_string(),
        updated_at: common_utils::date_time::now(),
    };
    
    db.update_remittance(remittance, final_update, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    Ok(())
}

/// Retrieve flow
#[instrument(skip_all)]
pub async fn remittance_retrieve_flow<Op>(
    state: &SessionState,
    req_state: &ReqState,
    merchant_account: &domain::MerchantAccount,
    profile: &domain::Profile,
    remittance_id: &str,
    req: api::remittances::RemittanceRetrieveRequest,
    _operation: Op,
) -> RouterResponse<api::remittances::RemittanceResponse>
where
    Op: RemittanceOperation,
{
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    let remittance_uuid = uuid::Uuid::parse_str(remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "remittance_id".to_string(),
            expected_format: "valid UUID".to_string(),
        })?;
    
    // Get remittance
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            &merchant_account.merchant_id,
            key_manager_state,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Force sync if requested
    if req.force_sync.unwrap_or(false) {
        sync_single_remittance(state, req_state, merchant_account, remittance.clone(), true)
            .await?;
    }
    
    // Transform to API response
    let response = transformers::foreign_to_api_remittance(
        state,
        remittance,
        merchant_account,
        profile,
    )
    .await?;
    
    Ok(services::ApplicationResponse::Json(response))
}

/// Update flow
#[instrument(skip_all)]
pub async fn remittance_update_flow<Op>(
    state: &SessionState,
    _req_state: &ReqState,
    merchant_account: &domain::MerchantAccount,
    profile: &domain::Profile,
    remittance_id: &str,
    req: api::remittances::RemittanceUpdateRequest,
    _operation: Op,
) -> RouterResponse<api::remittances::RemittanceResponse>
where
    Op: RemittanceOperation,
{
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    let remittance_uuid = uuid::Uuid::parse_str(remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "remittance_id".to_string(),
            expected_format: "valid UUID".to_string(),
        })?;
    
    // Get remittance
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            &merchant_account.merchant_id,
            key_manager_state,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Validate update is allowed
    validator::validate_remittance_updatable(&remittance)?;
    
    // Build update
    let update = storage::RemittanceUpdate::MetadataUpdate {
        metadata: req.metadata,
        purpose: req.reference,
        updated_at: common_utils::date_time::now(),
    };
    
    // Update remittance
    let updated_remittance = db
        .update_remittance(remittance, update, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Transform to API response
    let response = transformers::foreign_to_api_remittance(
        state,
        updated_remittance,
        merchant_account,
        profile,
    )
    .await?;
    
    Ok(services::ApplicationResponse::Json(response))
}

/// Cancel flow
#[instrument(skip_all)]
pub async fn remittance_cancel_flow<Op>(
    state: &SessionState,
    req_state: &ReqState,
    merchant_account: &domain::MerchantAccount,
    profile: &domain::Profile,
    remittance_id: &str,
    _operation: Op,
) -> RouterResponse<api::remittances::RemittanceResponse>
where
    Op: RemittanceOperation,
{
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    let remittance_uuid = uuid::Uuid::parse_str(remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "remittance_id".to_string(),
            expected_format: "valid UUID".to_string(),
        })?;
    
    // Get remittance
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            &merchant_account.merchant_id,
            key_manager_state,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Validate cancellation is allowed
    validator::validate_remittance_cancellable(&remittance)?;
    
    // Check if payment was made
    let payment_record = db
        .find_remittance_payment_by_remittance_id(&remittance_uuid, key_manager_state)
        .await
        .ok();
    
    // If payment exists and succeeded, initiate refund
    if let Some(payment) = payment_record {
        if let Some(payment_id) = payment.payment_id {
            if payment.status == Some("succeeded".to_string()) {
                // Initiate refund
                let refund_response = payments_core::refunds_create_core(
                    state.clone(),
                    req_state.clone(),
                    merchant_account.clone(),
                    profile.clone(),
                    payment_id,
                    api::refunds::RefundsCreateRequest {
                        amount: Some(remittance.amount),
                        reason: Some("Remittance cancelled".to_string()),
                        metadata: Some(serde_json::json!({
                            "remittance_id": remittance_id,
                            "type": "remittance_cancellation"
                        })),
                        ..Default::default()
                    },
                )
                .await?;
                
                router_env::logger::info!(
                    "Initiated refund {} for cancelled remittance {}",
                    refund_response.refund_id,
                    remittance_id
                );
            }
        }
    }
    
    // Update status to Cancelled
    let update = storage::RemittanceUpdate::StatusUpdate {
        status: storage::RemittanceStatus::Cancelled.to_string(),
        updated_at: common_utils::date_time::now(),
    };
    
    let cancelled_remittance = db
        .update_remittance(remittance, update, key_manager_state)
        .await
        .to_not_found_response(errors::ApiErrorResponse::RemittanceNotFound)?;
    
    // Transform to API response
    let response = transformers::foreign_to_api_remittance(
        state,
        cancelled_remittance,
        merchant_account,
        profile,
    )
    .await?;
    
    Ok(services::ApplicationResponse::Json(response))
}

/// Quote flow
#[instrument(skip_all)]
pub async fn remittance_quote_flow<Op>(
    state: &SessionState,
    _req_state: &ReqState,
    merchant_account: &domain::MerchantAccount,
    profile: &domain::Profile,
    req: api::remittances::RemittanceQuoteRequest,
    _operation: Op,
) -> RouterResponse<api::remittances::RemittanceQuoteResponse>
where
    Op: RemittanceOperation,
{
    // Validate currencies are supported
    validator::validate_currency_support(
        profile,
        &req.source_currency,
        &req.destination_currency,
    )?;
    
    // Get exchange rate from connector or fallback service
    let (rate, fee) = helpers::get_exchange_rate_with_fee(
        state,
        &req.source_currency,
        &req.destination_currency,
        req.amount.get_amount(),
        req.connector.as_ref(),
    )
    .await?;
    
    // Calculate amounts
    let source_amount = req.amount.get_amount();
    let fee_amount = fee.unwrap_or(0);
    let total_cost = source_amount + fee_amount;
    let destination_amount = helpers::calculate_destination_amount(source_amount, rate);
    
    let response = api::remittances::RemittanceQuoteResponse {
        source_currency: req.source_currency,
        destination_currency: req.destination_currency,
        source_amount: req.amount,
        destination_amount: common_utils::types::MinorUnit::new(destination_amount),
        rate: rate.to_f64().unwrap_or(0.0),
        fee: fee.map(common_utils::types::MinorUnit::new),
        total_cost: common_utils::types::MinorUnit::new(total_cost),
        estimated_delivery_time: Some(24), // Hours - connector specific
        rate_valid_until: Some(
            common_utils::date_time::now() + time::Duration::minutes(15)
        ),
        connector: req.connector.unwrap_or_else(|| "default".to_string()),
    };
    
    Ok(services::ApplicationResponse::Json(response))
}

/// Sync single remittance
pub async fn sync_single_remittance(
    state: &SessionState,
    req_state: &ReqState,
    merchant_account: &domain::MerchantAccount,
    mut remittance: storage::Remittance,
    force_sync: bool,
) -> CustomResult<SyncResult, errors::ApiErrorResponse> {
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    let mut payment_updated = false;
    let mut payout_updated = false;
    
    // Sync payment status if exists
    if let Ok(payment_record) = db
        .find_remittance_payment_by_remittance_id(&remittance.id, key_manager_state)
        .await
    {
        if let Some(payment_id) = payment_record.payment_id {
            // Sync payment status
            let payment_sync = payments_core::payments_core(
                state.clone(),
                req_state.clone(),
                merchant_account.clone(),
                api::enums::MerchantAccountIdentifier::MerchantId(merchant_account.merchant_id.clone()),
                payments_core::operations::PaymentStatus,
                payment_id.clone(),
                services::AuthFlow::Merchant,
                payments_core::CallConnectorAction::Trigger,
                None,
            )
            .await?;
            
            // Update payment record if status changed
            if Some(payment_sync.status.to_string()) != payment_record.status {
                payment_updated = true;
                let update = storage::RemittancePaymentUpdate {
                    status: Some(payment_sync.status.to_string()),
                    updated_at: Some(common_utils::date_time::now()),
                    ..Default::default()
                };
                
                db.update_remittance_payment_by_remittance_id(
                    &remittance.id,
                    update,
                    key_manager_state,
                )
                .await?;
                
                // Update remittance status if payment completed
                if payment_sync.status == api_models::enums::IntentStatus::Succeeded
                    && remittance.status == storage::RemittanceStatus::PaymentInitiated.to_string()
                {
                    remittance = db
                        .update_remittance(
                            remittance,
                            storage::RemittanceUpdate::StatusUpdate {
                                status: storage::RemittanceStatus::PaymentProcessed.to_string(),
                                updated_at: common_utils::date_time::now(),
                            },
                            key_manager_state,
                        )
                        .await?;
                }
            }
        }
    }
    
    // Sync payout status if exists
    if let Ok(payout_record) = db
        .find_remittance_payout_by_remittance_id(&remittance.id, key_manager_state)
        .await
    {
        if let Some(payout_id) = payout_record.payout_id {
            // Sync payout status
            let payout_sync = payouts_core::payouts_retrieve_core(
                state.clone(),
                req_state.clone(),
                merchant_account.clone(),
                payout_id.clone(),
            )
            .await?;
            
            // Update payout record if status changed
            if Some(payout_sync.status.to_string()) != payout_record.status {
                payout_updated = true;
                let update = storage::RemittancePayoutUpdate {
                    status: Some(payout_sync.status.to_string()),
                    updated_at: Some(common_utils::date_time::now()),
                    ..Default::default()
                };
                
                db.update_remittance_payout_by_remittance_id(
                    &remittance.id,
                    update,
                    key_manager_state,
                )
                .await?;
                
                // Update remittance status based on payout
                let new_status = match payout_sync.status {
                    api_models::enums::PayoutStatus::Success => {
                        Some(storage::RemittanceStatus::Completed)
                    }
                    api_models::enums::PayoutStatus::Failed => {
                        Some(storage::RemittanceStatus::Failed)
                    }
                    _ => None,
                };
                
                if let Some(status) = new_status {
                    remittance = db
                        .update_remittance(
                            remittance,
                            storage::RemittanceUpdate::StatusUpdate {
                                status: status.to_string(),
                                updated_at: common_utils::date_time::now(),
                            },
                            key_manager_state,
                        )
                        .await?;
                }
            }
        }
    }
    
    Ok(SyncResult {
        remittance,
        payment_updated,
        payout_updated,
    })
}

/// Process webhook
pub async fn process_remittance_webhook(
    state: &SessionState,
    req_state: &ReqState,
    remittance: storage::Remittance,
    webhook_data: helpers::WebhookData,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    // Update based on webhook type
    match webhook_data.webhook_type {
        helpers::WebhookType::Payment => {
            // Update payment record
            let update = storage::RemittancePaymentUpdate {
                status: Some(webhook_data.status.clone()),
                connector_txn_id: webhook_data.connector_reference,
                updated_at: Some(common_utils::date_time::now()),
                ..Default::default()
            };
            
            db.update_remittance_payment_by_remittance_id(
                &remittance.id,
                update,
                key_manager_state,
            )
            .await?;
            
            // Update remittance status if payment completed
            if webhook_data.status == "succeeded" {
                let status_update = storage::RemittanceUpdate::StatusUpdate {
                    status: storage::RemittanceStatus::PaymentProcessed.to_string(),
                    updated_at: common_utils::date_time::now(),
                };
                
                let updated_remittance = db
                    .update_remittance(remittance, status_update, key_manager_state)
                    .await?;
                
                // Initiate payout
                tokio::spawn(async move {
                    if let Err(e) = initiate_remittance_payout(
                        state.clone(),
                        req_state.clone(),
                        updated_remittance.id,
                    )
                    .await
                    {
                        router_env::logger::error!(
                            ?e,
                            "Failed to initiate payout for remittance {}",
                            updated_remittance.id
                        );
                    }
                });
            }
        }
        
        helpers::WebhookType::Payout => {
            // Update payout record
            let update = storage::RemittancePayoutUpdate {
                status: Some(webhook_data.status.clone()),
                connector_txn_id: webhook_data.connector_reference,
                updated_at: Some(common_utils::date_time::now()),
                ..Default::default()
            };
            
            db.update_remittance_payout_by_remittance_id(
                &remittance.id,
                update,
                key_manager_state,
            )
            .await?;
            
            // Update remittance status based on payout status
            let remittance_status = match webhook_data.status.as_str() {
                "succeeded" | "completed" => storage::RemittanceStatus::Completed,
                "failed" => storage::RemittanceStatus::Failed,
                _ => return Ok(()),
            };
            
            let status_update = storage::RemittanceUpdate::StatusUpdate {
                status: remittance_status.to_string(),
                updated_at: common_utils::date_time::now(),
            };
            
            db.update_remittance(remittance, status_update, key_manager_state)
                .await?;
        }
    }
    
    Ok(())
}

/// Auto-initiate payment
async fn initiate_auto_payment(
    state: SessionState,
    req_state: ReqState,
    remittance_id: String,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Implementation for auto-processing payment
    // This would typically use stored payment method or default funding source
    router_env::logger::info!("Auto-processing payment for remittance {}", remittance_id);
    
    // TODO: Implement auto payment logic
    
    Ok(())
}

pub struct SyncResult {
    pub remittance: storage::Remittance,
    pub payment_updated: bool,
    pub payout_updated: bool,
}