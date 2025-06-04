//! Core remittance processing logic
//! 
//! This module orchestrates remittance operations by combining
//! the existing payment and payout infrastructure.

use api_models::remittances as api;
use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, Encode, OptionExt, ValueExt},
    id_type::{CustomerId, PaymentId},
    types::MinorUnit,
};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
use uuid::Uuid;

use super::{
    errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    payments::{self, PaymentData},
    payouts::{self, PayoutData},
    utils as core_utils,
};
use crate::{
    db::StorageInterface,
    routes::SessionState,
    services::{self, ApplicationResponse},
    types::{
        api as api_types,
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt as _,
};

/// Main remittance data structure that holds all related information
#[derive(Clone)]
pub struct RemittanceData {
    pub remittance: storage::Remittance,
    pub payment: Option<storage::RemittancePayment>,
    pub payout: Option<storage::RemittancePayout>,
    pub merchant_account: domain::MerchantAccount,
    pub profile: domain::Profile,
    pub key_store: domain::MerchantKeyStore,
}

/// Create a new remittance
#[instrument(skip_all)]
pub async fn create_remittance(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    req: api::RemittanceRequest,
) -> RouterResponse<api::RemittanceResponse> {
    let db = &state.store;
    let merchant_id = merchant_context.get_merchant_account().get_id();
    let key_store = merchant_context.get_merchant_key_store();
    
    // Validate profile
    let profile_id = req.profile_id.clone()
        .or(merchant_context.get_merchant_account().default_profile.clone())
        .get_required_value("profile_id")?;
    
    let profile = core_utils::validate_and_get_business_profile(
        db,
        Some(&profile_id),
        merchant_id,
        key_store,
    )
    .await?
    .get_required_value("profile")?;
    
    // Generate remittance ID
    let remittance_id = req.remittance_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    let remittance_uuid = Uuid::parse_str(&remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid remittance_id format".to_string(),
        })?;
    
    // Create remittance entry
    let new_remittance = storage::RemittanceNew {
        id: remittance_uuid,
        merchant_id: merchant_id.clone(),
        profile_id: profile_id.clone(),
        amount: req.amount.get_amount_as_i64(),
        source_currency: req.source_currency.to_string(),
        destination_currency: req.destination_currency.to_string(),
        source_amount: Some(req.amount.get_amount_as_i64()),
        destination_amount: None,
        exchange_rate: None,
        reference: req.reference.clone(),
        purpose: req.purpose.as_ref().map(|p| p.to_string()),
        status: api::RemittanceStatus::Created.to_string(),
        failure_reason: None,
        sender_details: req.sender_details.encode_to_value()?,
        beneficiary_details: req.beneficiary_details.encode_to_value()?,
        return_url: req.return_url.clone(),
        metadata: req.metadata.clone(),
        connector: req.connector.clone().unwrap_or_else(|| "default".to_string()),
        client_secret: Some(core_utils::generate_id(
            consts::ID_LENGTH,
            "rem_secret",
        )),
        remittance_date: req.remittance_date.parse()
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid remittance_date format".to_string(),
            })?,
        created_at: Some(common_utils::date_time::now()),
        updated_at: Some(common_utils::date_time::now()),
    };
    
    let remittance = db
        .insert_remittance(new_remittance)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateRequest)?;
    
    // Create payment and payout tracking entries
    let payment_entry = storage::RemittancePaymentNew {
        remittance_id: remittance_uuid,
        payment_id: None,
        connector_txn_id: None,
        status: None,
        auth_type: None,
        created_at: Some(common_utils::date_time::now()),
        updated_at: Some(common_utils::date_time::now()),
    };
    
    let payout_entry = storage::RemittancePayoutNew {
        remittance_id: remittance_uuid,
        payout_id: None,
        connector_txn_id: None,
        status: None,
        remittance_method: None,
        created_at: Some(common_utils::date_time::now()),
        updated_at: Some(common_utils::date_time::now()),
    };
    
    let payment = db.insert_remittance_payment(payment_entry).await?;
    let payout = db.insert_remittance_payout(payout_entry).await?;
    
    // Auto-process if requested
    if req.auto_process.unwrap_or(false) && req.sender_details.payment_method_data.is_some() {
        let remittance_data = RemittanceData {
            remittance: remittance.clone(),
            payment: Some(payment),
            payout: Some(payout),
            merchant_account: merchant_context.get_merchant_account().clone(),
            profile,
            key_store: key_store.clone(),
        };
        
        Box::pin(process_remittance_payment(
            state,
            merchant_context,
            remittance_data,
            req.sender_details.payment_method_data,
        ))
        .await?;
    }
    
    build_remittance_response(&remittance, payment, payout)
}

/// Fund a remittance with payment
#[instrument(skip_all)]
pub async fn fund_remittance(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    remittance_id: String,
    req: api::RemittancePayRequest,
) -> RouterResponse<api::RemittanceResponse> {
    let db = &state.store;
    let remittance_uuid = Uuid::parse_str(&remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid remittance_id format".to_string(),
        })?;
    
    // Fetch remittance and validate status
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            merchant_context.get_merchant_account().get_id(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceNotFound)?;
    
    validate_remittance_status_for_payment(&remittance)?;
    
    let payment = db
        .find_remittance_payment_by_id(&remittance_uuid)
        .await?;
    
    let payout = db
        .find_remittance_payout_by_id(&remittance_uuid)
        .await?;
    
    let profile = core_utils::get_profile(
        db,
        &remittance.profile_id,
        merchant_context.get_merchant_key_store(),
    )
    .await?;
    
    let remittance_data = RemittanceData {
        remittance,
        payment: Some(payment),
        payout: Some(payout),
        merchant_account: merchant_context.get_merchant_account().clone(),
        profile,
        key_store: merchant_context.get_merchant_key_store().clone(),
    };
    
    let payment_method_data = req.payment_method_data
        .get_required_value("payment_method_data")?;
    
    Box::pin(process_remittance_payment(
        state,
        merchant_context,
        remittance_data,
        Some(payment_method_data),
    ))
    .await
}

/// Process the payment part of remittance
async fn process_remittance_payment(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    mut remittance_data: RemittanceData,
    payment_method_data: Option<api_types::PaymentMethodDataRequest>,
) -> RouterResult<api::RemittanceResponse> {
    let remittance = &remittance_data.remittance;
    
    // Build payment request
    let payment_request = api_types::PaymentsRequest {
        payment_id: Some(api_types::PaymentIdType::PaymentIntentId(
            format!("rem_pay_{}", remittance.id),
        )),
        amount: Some(api_types::Amount::from(MinorUnit::new(remittance.amount))),
        currency: Some(remittance.source_currency.parse().change_context(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid source currency".to_string(),
            },
        )?),
        payment_method_data,
        confirm: Some(true),
        customer_id: remittance_data.remittance
            .sender_details
            .as_ref()
            .get("customer_id")
            .and_then(|v| v.as_str())
            .map(|id| CustomerId::try_from(id.to_string()).ok())
            .flatten(),
        return_url: remittance.return_url.clone()
            .and_then(|url| url::Url::parse(&url).ok()),
        metadata: Some(serde_json::json!({
            "remittance_id": remittance.id.to_string(),
            "type": "remittance_payment",
            "reference": remittance.reference,
        })),
        ..Default::default()
    };
    
    // Process payment through existing payment flow
    let payment_response = Box::pin(payments::payments_core::
        api_types::Authorize,
        api_types::PaymentsResponse,
        _,
        _,
        _,
        _,
    >(
        state.clone(),
        merchant_context.clone(),
        payments::PaymentConfirm,
        payment_request,
        services::AuthFlow::Merchant,
        payments::CallConnectorAction::Trigger,
        None,
        HeaderPayload::default(),
    ))
    .await?;
    
    // Extract payment response
    let payment_info = match payment_response {
        ApplicationResponse::Json(resp) => resp,
        _ => Err(errors::ApiErrorResponse::InternalServerError)?,
    };
    
    // Update remittance payment record
    let payment_update = storage::RemittancePaymentUpdate {
        payment_id: Some(payment_info.payment_id.clone()),
        connector_txn_id: payment_info.connector_transaction_id.clone(),
        status: Some(payment_info.status.to_string()),
        auth_type: payment_info.authentication_type.map(|a| a.to_string()),
        updated_at: Some(common_utils::date_time::now()),
    };
    
    remittance_data.payment = Some(
        state
            .store
            .update_remittance_payment(&remittance.id, payment_update)
            .await?,
    );
    
    // Update remittance status based on payment result
    let new_status = match payment_info.status {
        api_types::IntentStatus::Succeeded => {
            // If payment succeeded, initiate payout
            api::RemittanceStatus::PaymentProcessed
        }
        api_types::IntentStatus::Failed | api_types::IntentStatus::Cancelled => {
            api::RemittanceStatus::Failed
        }
        _ => api::RemittanceStatus::PaymentInitiated,
    };
    
    let remittance_update = storage::RemittanceUpdate::StatusUpdate {
        status: new_status.to_string(),
        updated_at: common_utils::date_time::now(),
    };
    
    remittance_data.remittance = state
        .store
        .update_remittance(&remittance.id, remittance_update)
        .await?;
    
    // If payment succeeded, automatically initiate payout
    if new_status == api::RemittanceStatus::PaymentProcessed {
        return Box::pin(process_remittance_payout(state, merchant_context, remittance_data)).await;
    }
    
    build_remittance_response(
        &remittance_data.remittance,
        remittance_data.payment,
        remittance_data.payout,
    )
}

/// Process the payout part of remittance
async fn process_remittance_payout(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    mut remittance_data: RemittanceData,
) -> RouterResult<api::RemittanceResponse> {
    let remittance = &remittance_data.remittance;
    let beneficiary = remittance.beneficiary_details
        .parse_value::<api::BeneficiaryDetails>("BeneficiaryDetails")?;
    
    // Build payout request
    let payout_request = api_types::PayoutCreateRequest {
        payout_id: Some(format!("rem_po_{}", remittance.id)),
        amount: Some(api_types::Amount::from(MinorUnit::new(
            remittance.destination_amount.unwrap_or(remittance.amount),
        ))),
        currency: Some(remittance.destination_currency.parse().change_context(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid destination currency".to_string(),
            },
        )?),
        customer_id: beneficiary.customer_id,
        payout_type: beneficiary.payout_details
            .as_ref()
            .map(|details| match details {
                api::PayoutMethodData::BankTransfer(_) => api_types::PayoutType::Bank,
                api::PayoutMethodData::Card(_) => api_types::PayoutType::Card,
                api::PayoutMethodData::Wallet(_) => api_types::PayoutType::Wallet,
                api::PayoutMethodData::CashPickup(_) => api_types::PayoutType::Bank, // Fallback
            }),
        confirm: Some(true),
        metadata: Some(serde_json::json!({
            "remittance_id": remittance.id.to_string(),
            "type": "remittance_payout",
            "reference": remittance.reference,
        })),
        ..Default::default()
    };
    
    // Process payout through existing payout flow
    let payout_response = Box::pin(payouts::payouts_create_core(
        state.clone(),
        merchant_context.clone(),
        payout_request,
    ))
    .await?;
    
    // Extract payout response
    let payout_info = match payout_response {
        ApplicationResponse::Json(resp) => resp,
        _ => Err(errors::ApiErrorResponse::InternalServerError)?,
    };
    
    // Update remittance payout record
    let payout_update = storage::RemittancePayoutUpdate {
        payout_id: Some(payout_info.payout_id.clone()),
        connector_txn_id: payout_info.connector_transaction_id.clone(),
        status: Some(payout_info.status.to_string()),
        remittance_method: beneficiary.payout_details
            .as_ref()
            .map(|d| d.get_method_type().to_string()),
        updated_at: Some(common_utils::date_time::now()),
    };
    
    remittance_data.payout = Some(
        state
            .store
            .update_remittance_payout(&remittance.id, payout_update)
            .await?,
    );
    
    // Update remittance status based on payout result
    let new_status = match payout_info.status {
        api_types::PayoutStatus::Success => api::RemittanceStatus::Completed,
        api_types::PayoutStatus::Failed | api_types::PayoutStatus::Cancelled => {
            api::RemittanceStatus::Failed
        }
        _ => api::RemittanceStatus::PayoutInitiated,
    };
    
    let remittance_update = storage::RemittanceUpdate::StatusUpdate {
        status: new_status.to_string(),
        updated_at: common_utils::date_time::now(),
    };
    
    remittance_data.remittance = state
        .store
        .update_remittance(&remittance.id, remittance_update)
        .await?;
    
    build_remittance_response(
        &remittance_data.remittance,
        remittance_data.payment,
        remittance_data.payout,
    )
}

/// Retrieve a remittance
#[instrument(skip_all)]
pub async fn retrieve_remittance(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    remittance_id: String,
    req: api::RemittancesRetrieveRequest,
) -> RouterResponse<api::RemittanceResponse> {
    let db = &state.store;
    let remittance_uuid = Uuid::parse_str(&remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid remittance_id format".to_string(),
        })?;
    
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            merchant_context.get_merchant_account().get_id(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceNotFound)?;
    
    // Verify client secret if provided
    if let Some(client_secret) = &req.client_secret {
        if remittance.client_secret.as_ref() != Some(client_secret) {
            return Err(errors::ApiErrorResponse::UnauthorizedAccess)?;
        }
    }
    
    let payment = db.find_remittance_payment_by_id(&remittance_uuid).await?;
    let payout = db.find_remittance_payout_by_id(&remittance_uuid).await?;
    
    // Force sync if requested
    if req.force_sync.unwrap_or(false) {
        // Sync payment status
        if let Some(payment_id) = &payment.payment_id {
            let payment_sync_req = api_types::PaymentsRetrieveRequest {
                resource_id: api_types::PaymentIdType::PaymentIntentId(payment_id.clone()),
                force_sync: true,
                ..Default::default()
            };
            
            let _ = Box::pin(payments::payments_core::
                api_types::PSync,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                _,
            >(
                state.clone(),
                merchant_context.clone(),
                payments::PaymentStatus,
                payment_sync_req,
                services::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            ))
            .await;
        }
        
        // Sync payout status
        if let Some(payout_id) = &payout.payout_id {
            let payout_sync_req = api_types::PayoutRetrieveRequest {
                payout_id: payout_id.clone(),
                force_sync: Some(true),
                ..Default::default()
            };
            
            let _ = Box::pin(payouts::payouts_retrieve_core(
                state.clone(),
                merchant_context.clone(),
                Some(remittance.profile_id.clone()),
                payout_sync_req,
            ))
            .await;
        }
        
        // Re-fetch updated records
        let payment = db.find_remittance_payment_by_id(&remittance_uuid).await?;
        let payout = db.find_remittance_payout_by_id(&remittance_uuid).await?;
        
        return build_remittance_response(&remittance, payment, payout);
    }
    
    build_remittance_response(&remittance, payment, payout)
}

/// Update a remittance
#[instrument(skip_all)]
pub async fn update_remittance(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    remittance_id: String,
    req: api::RemittanceUpdateRequest,
) -> RouterResponse<api::RemittanceResponse> {
    let db = &state.store;
    let remittance_uuid = Uuid::parse_str(&remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid remittance_id format".to_string(),
        })?;
    
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            merchant_context.get_merchant_account().get_id(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceNotFound)?;
    
    // Only allow updates in certain states
    validate_remittance_status_for_update(&remittance)?;
    
    let update = storage::RemittanceUpdate::MetadataUpdate {
        metadata: req.metadata,
        purpose: req.reference.map(|_| req.reference.unwrap_or_default()),
        updated_at: common_utils::date_time::now(),
    };
    
    let updated_remittance = db.update_remittance(&remittance_uuid, update).await?;
    
    let payment = db.find_remittance_payment_by_id(&remittance_uuid).await?;
    let payout = db.find_remittance_payout_by_id(&remittance_uuid).await?;
    
    build_remittance_response(&updated_remittance, payment, payout)
}

/// List remittances
#[instrument(skip_all)]
pub async fn list_remittances(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    req: api::RemittanceListRequest,
) -> RouterResponse<api::RemittanceListResponse> {
    let db = &state.store;
    let merchant_id = merchant_context.get_merchant_account().get_id();
    
    let constraints = storage::RemittanceListConstraints {
        merchant_id: merchant_id.clone(),
        status: req.status.map(|statuses| {
            statuses.into_iter().map(|s| s.to_string()).collect()
        }),
        connector: req.connector,
        source_currency: req.source_currency.map(|c| c.to_string()),
        destination_currency: req.destination_currency.map(|c| c.to_string()),
        time_range: req.time_range,
        limit: req.limit.unwrap_or(10).min(100) as i64,
        offset: req.offset.unwrap_or(0) as i64,
    };
    
    let (remittances, total_count) = db.list_remittances(constraints).await?;
    
    let mut response_data = Vec::with_capacity(remittances.len());
    
    for remittance in remittances {
        let payment = db.find_remittance_payment_by_id(&remittance.id).await?;
        let payout = db.find_remittance_payout_by_id(&remittance.id).await?;
        
        response_data.push(build_remittance_response(&remittance, payment, payout)?);
    }
    
    Ok(ApplicationResponse::Json(api::RemittanceListResponse {
        total_count,
        count: response_data.len(),
        data: response_data,
    }))
}

/// Cancel a remittance
#[instrument(skip_all)]
pub async fn cancel_remittance(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    remittance_id: String,
) -> RouterResponse<api::RemittanceResponse> {
    let db = &state.store;
    let remittance_uuid = Uuid::parse_str(&remittance_id)
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid remittance_id format".to_string(),
        })?;
    
    let remittance = db
        .find_remittance_by_id_merchant_id(
            &remittance_uuid,
            merchant_context.get_merchant_account().get_id(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceNotFound)?;
    
    validate_remittance_status_for_cancellation(&remittance)?;
    
    let payment = db.find_remittance_payment_by_id(&remittance_uuid).await?;
    let payout = db.find_remittance_payout_by_id(&remittance_uuid).await?;
    
    // Cancel payment if exists and not completed
    if let Some(payment_id) = &payment.payment_id {
        if payment.status.as_deref() != Some("succeeded") {
            let cancel_req = api_types::PaymentsCancelRequest {
                payment_id: payment_id.clone(),
                cancellation_reason: Some("Remittance cancelled".to_string()),
                ..Default::default()
            };
            
            let _ = Box::pin(payments::payments_core::
                api_types::Void,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                _,
            >(
                state.clone(),
                merchant_context.clone(),
                payments::PaymentCancel,
                cancel_req,
                services::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            ))
            .await;
        }
    }
    
    // Cancel payout if exists and not completed
    if let Some(payout_id) = &payout.payout_id {
        if payout.status.as_deref() != Some("success") {
            let cancel_req = api_types::PayoutActionRequest {
                payout_id: payout_id.clone(),
                ..Default::default()
            };
            
            let _ = Box::pin(payouts::payouts_cancel_core(
                state.clone(),
                merchant_context.clone(),
                cancel_req,
            ))
            .await;
        }
    }
    
    // Update remittance status
    let update = storage::RemittanceUpdate::StatusUpdate {
        status: api::RemittanceStatus::Cancelled.to_string(),
        updated_at: common_utils::date_time::now(),
    };
    
    let updated_remittance = db.update_remittance(&remittance_uuid, update).await?;
    
    build_remittance_response(&updated_remittance, payment, payout)
}

/// Get exchange rate quote
#[instrument(skip_all)]
pub async fn get_remittance_quote(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    req: api::RemittanceQuoteRequest,
) -> RouterResponse<api::RemittanceQuoteResponse> {
    let profile_id = merchant_context
        .get_merchant_account()
        .default_profile
        .clone()
        .get_required_value("profile_id")?;
    
    let profile = core_utils::get_profile(
        &state.store,
        &profile_id,
        merchant_context.get_merchant_key_store(),
    )
    .await?;
    
    // Get exchange rate from configured provider
    let (rate, fee, delivery_time) = get_exchange_rate_and_fees(
        &state,
        &profile,
        &req.source_currency,
        &req.destination_currency,
        req.amount,
        req.connector.as_deref(),
    )
    .await?;
    
    let destination_amount = calculate_destination_amount(req.amount, rate, fee);
    let total_cost = req.amount + fee.unwrap_or(MinorUnit::zero());
    
    Ok(ApplicationResponse::Json(api::RemittanceQuoteResponse {
        source_currency: req.source_currency,
        destination_currency: req.destination_currency,
        source_amount: req.amount,
        destination_amount,
        rate,
        fee,
        total_cost,
        estimated_delivery_time: Some(delivery_time),
        rate_valid_until: Some(common_utils::date_time::now() + Duration::minutes(15)),
        connector: req.connector.unwrap_or_else(|| "default".to_string()),
    }))
}

#[instrument(skip_all)]
pub async fn sync_remittances(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    req: api::RemittanceSyncRequest,
) -> RouterResponse<api::RemittanceSyncResponse> {
    let db = &state.store;

    // 1) Fetch remittances in ‘in‐flight’ states
    let constraints = storage::RemittanceSyncConstraints {
        merchant_id: merchant_context.get_merchant_account().get_id().clone(),
        time_range: req.time_range,
        status_filter: vec![
            api::RemittanceStatus::PaymentInitiated.to_string(),
            api::RemittanceStatus::PaymentProcessed.to_string(),
            api::RemittanceStatus::PayoutInitiated.to_string(),
        ],
    };
    let remittances = db
        .find_remittances_for_sync(constraints)
        .await
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let mut sync_results = Vec::with_capacity(remittances.len());

    for mut rem in remittances {
        // load existing payment & payout records
        let mut payment = db
            .find_remittance_payment_by_id(&rem.id)
            .await
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let mut payout = db
            .find_remittance_payout_by_id(&rem.id)
            .await
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let previous_status = api::RemittanceStatus::from_string(&rem.status)
            .unwrap_or(api::RemittanceStatus::Failed)
            .to_string();
        let mut payment_updated = false;
        let mut payout_updated = false;

        // 2) Sync payment if it has an ID and is not final
        if let Some(pay_id) = &payment.payment_id {
            if !matches!(payment.status.as_deref(), Some(s) if s == "succeeded" || s == "failed") {
                let payment_sync_req = api_types::PaymentsRetrieveRequest {
                    resource_id: api_types::PaymentIdType::PaymentIntentId(pay_id.clone()),
                    force_sync: true,
                    ..Default::default()
                };

                let sync_resp = Box::pin(
                    payments::payments_core::<
                        api_types::PaymentsRetrieveRequest,
                        api_types::PaymentsResponse,
                    >(
                        state.clone(),
                        merchant_context.clone(),
                        payments::PaymentStatus,
                        payment_sync_req,
                        services::AuthFlow::Merchant,
                        payments::CallConnectorAction::Trigger,
                        None,
                        Default::default(),
                    ),
                )
                .await?
                .into_report()
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

                if sync_resp.status.to_string() != payment.status.clone().unwrap_or_default() {
                    payment_updated = true;
                    payment = db
                        .update_remittance_payment(
                            &rem.id,
                            storage::RemittancePaymentUpdate {
                                status: Some(sync_resp.status.to_string()),
                                updated_at: Some(common_utils::date_time::now()),
                                ..Default::default()
                            },
                        )
                        .await
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)?;
                }
            }
        }

        // 3) Sync payout if it has an ID and is not final
        if let Some(po_id) = &payout.payout_id {
            if !matches!(payout.status.as_deref(), Some(s) if s == "success" || s == "failed") {
                let payout_sync_req = api_types::PayoutRetrieveRequest {
                    payout_id: po_id.clone(),
                    force_sync: Some(true),
                    ..Default::default()
                };

                let sync_resp = Box::pin(
                    payouts::payouts_retrieve_core(
                        state.clone(),
                        merchant_context.clone(),
                        payout_sync_req,
                    ),
                )
                .await?
                .into_report()
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

                if sync_resp.status.to_string() != payout.status.clone().unwrap_or_default() {
                    payout_updated = true;
                    payout = db
                        .update_remittance_payout(
                            &rem.id,
                            storage::RemittancePayoutUpdate {
                                status: Some(sync_resp.status.to_string()),
                                updated_at: Some(common_utils::date_time::now()),
                                ..Default::default()
                            },
                        )
                        .await
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)?;
                }
            }
        }

        // 4) If anything changed, update the remittance’s overall status
        if payment_updated || payout_updated {
            let new_status = match (payment.status.clone(), payout.status.clone()) {
                (Some(ps), _) if ps == api::IntentStatus::Succeeded.to_string() => {
                    if payout.status.as_deref() == Some(api::PayoutStatus::Success.to_string()) {
                        api::RemittanceStatus::Completed
                    } else {
                        api::RemittanceStatus::PaymentProcessed
                    }
                }
                _ => api::RemittanceStatus::Failed,
            };
            rem = db
                .update_remittance(
                    &rem.id,
                    storage::RemittanceUpdate::StatusUpdate {
                        status: new_status.to_string(),
                        updated_at: common_utils::date_time::now(),
                    },
                )
                .await
                .into_report()
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
        }

        // 5) Collect result
        sync_results.push(api::RemittanceSyncResult {
            remittance_id: rem.remittance_id.clone(),
            previous_status,
            current_status: rem.status.clone(),
            payment_updated,
            payout_updated,
        });
    }

    // 6) Return
    Ok(services::ApplicationResponse::Json(api::RemittanceSyncResponse {
        results: sync_results,
    }))
}
