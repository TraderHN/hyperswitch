//! Data transformation functions for remittances

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use serde_json::json;

use crate::{
    core::errors,
    types::{
        api::{self, payments as payment_types, payouts as payout_types, remittances as remittances_api},
        domain, storage,
    },
    SessionState,
};

/// Transform storage remittance to API response
pub async fn foreign_to_api_remittance(
    state: &SessionState,
    remittance: storage::Remittance,
    merchant_account: &domain::MerchantAccount,
    profile: &domain::Profile,
) -> CustomResult<remittances_api::RemittanceResponse, errors::ApiErrorResponse> {
    let db = &state.store;
    let key_manager_state = &state.into_inner();
    
    // Get payment and payout records
    let payment_record = db
        .find_remittance_payment_by_remittance_id(&remittance.id, key_manager_state)
        .await
        .ok();
    
    let payout_record = db
        .find_remittance_payout_by_remittance_id(&remittance.id, key_manager_state)
        .await
        .ok();
    
    // Parse sender and beneficiary details
    let sender_details: Option<api::remittances::SenderDetails> = remittance
        .sender_details
        .parse_value("SenderDetails")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    
    let beneficiary_details: Option<api::remittances::BeneficiaryDetails> = remittance
        .beneficiary_details
        .parse_value("BeneficiaryDetails")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    
    // Build exchange rate info
    let exchange_rate_info = remittance.exchange_rate.map(|rate| {
        api::remittances::ExchangeRateInfo {
            rate: rate.to_f64().unwrap_or(0.0),
            markup: None, // TODO: Calculate markup if applicable
            source_currency: remittance.source_currency.parse().unwrap_or_default(),
            destination_currency: remittance.destination_currency.parse().unwrap_or_default(),
            valid_until: None, // TODO: Add rate validity
        }
    });
    
    // Parse status
    let status = remittance
        .status
        .parse()
        .unwrap_or(api::remittances::RemittanceStatus::Failed);
    
    Ok(remittances_api::RemittanceResponse {
        remittance_id: remittance.id.to_string(),
        merchant_id: remittance.merchant_id,
        profile_id: remittance.profile_id,
        amount: common_utils::types::MinorUnit::new(remittance.amount),
        source_currency: remittance.source_currency.parse().unwrap_or_default(),
        destination_currency: remittance.destination_currency.parse().unwrap_or_default(),
        source_amount: common_utils::types::MinorUnit::new(
            remittance.source_amount.unwrap_or(remittance.amount)
        ),
        destination_amount: common_utils::types::MinorUnit::new(
            remittance.destination_amount.unwrap_or(0)
        ),
        exchange_rate: exchange_rate_info,
        sender_details,
        beneficiary_details,
        remittance_date: remittance.remittance_date.to_string(),
        reference: remittance.reference,
        purpose: remittance.purpose.and_then(|p| p.parse().ok()),
        status,
        failure_reason: remittance.failure_reason,
        return_url: remittance.return_url,
        metadata: remittance.metadata,
        connector: remittance.connector,
        client_secret: remittance.client_secret,
        payment_id: payment_record
            .as_ref()
            .and_then(|p| p.payment_id.clone())
            .map(|id| id.into()),
        payout_id: payout_record.as_ref().and_then(|p| p.payout_id.clone()),
        payment_connector_transaction_id: payment_record.and_then(|p| p.connector_txn_id),
        payout_connector_transaction_id: payout_record.and_then(|p| p.connector_txn_id),
        compliance_status: None, // TODO: Add compliance status
        required_documents: None, // TODO: Add required documents
        estimated_delivery_time: Some("24 hours".to_string()), // TODO: Make dynamic
        actual_delivery_time: None, // TODO: Calculate from completion time
        created_at: remittance.created_at,
        updated_at: remittance.updated_at,
    })
}

/// Transform storage remittance to API response (simple version)
pub fn storage_to_api_remittance(
    remittance: storage::Remittance,
) -> CustomResult<remittances_api::RemittanceResponse, errors::ApiErrorResponse> {
    // Simplified version without payment/payout lookups
    // Used for manual updates and other operations where full details aren't needed
    
    let status = remittance
        .status
        .parse()
        .unwrap_or(api::remittances::RemittanceStatus::Failed);
    
    Ok(remittances_api::RemittanceResponse {
        remittance_id: remittance.id.to_string(),
        merchant_id: remittance.merchant_id,
        profile_id: remittance.profile_id,
        amount: common_utils::types::MinorUnit::new(remittance.amount),
        source_currency: remittance.source_currency.parse().unwrap_or_default(),
        destination_currency: remittance.destination_currency.parse().unwrap_or_default(),
        source_amount: common_utils::types::MinorUnit::new(
            remittance.source_amount.unwrap_or(remittance.amount)
        ),
        destination_amount: common_utils::types::MinorUnit::new(
            remittance.destination_amount.unwrap_or(0)
        ),
        exchange_rate: None,
        sender_details: None,
        beneficiary_details: None,
        remittance_date: remittance.remittance_date.to_string(),
        reference: remittance.reference,
        purpose: remittance.purpose.and_then(|p| p.parse().ok()),
        status,
        failure_reason: remittance.failure_reason,
        return_url: remittance.return_url,
        metadata: remittance.metadata,
        connector: remittance.connector,
        client_secret: remittance.client_secret,
        payment_id: None,
        payout_id: None,
        payment_connector_transaction_id: None,
        payout_connector_transaction_id: None,
        compliance_status: None,
        required_documents: None,
        estimated_delivery_time: Some("24 hours".to_string()),
        actual_delivery_time: None,
        created_at: remittance.created_at,
        updated_at: remittance.updated_at,
    })
}

/// Transform remittance pay request to payment request
pub fn remittance_to_payment_request(
    remittance: &storage::Remittance,
    pay_request: api::remittances::RemittancePayRequest,
) -> CustomResult<payment_types::PaymentsRequest, errors::ApiErrorResponse> {
    // Parse sender details to get customer info
    let sender_details: api::remittances::SenderDetails = remittance
        .sender_details
        .parse_value("SenderDetails")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    
    let mut payment_request = payment_types::PaymentsRequest {
        amount: Some(api::Amount {
            value: remittance.amount,
            currency: remittance.source_currency.parse().unwrap_or_default(),
        }),
        capture_method: Some(api_models::enums::CaptureMethod::Automatic),
        payment_method_data: pay_request.payment_method_data,
        billing: pay_request.billing,
        confirm: pay_request.confirm.unwrap_or(true),
        return_url: pay_request
            .return_url
            .or(remittance.return_url.clone())
            .and_then(|u| url::Url::parse(&u).ok()),
        setup_future_usage: pay_request.setup_future_usage,
        browser_info: pay_request.browser_info,
        statement_descriptor_name: Some("REMITTANCE".to_string()),
        statement_descriptor_suffix: Some(remittance.reference.clone()),
        metadata: Some(json!({
            "remittance_id": remittance.id.to_string(),
            "type": "remittance_funding",
            "reference": remittance.reference,
        })),
        ..Default::default()
    };
    
    // Set customer details
    if let Some(customer_id) = sender_details.customer_id {
        payment_request.customer_id = Some(customer_id);
    } else {
        // Create customer details from sender info
        payment_request.customer = Some(payment_types::CustomerDetails {
            id: None,
            name: Some(sender_details.name.into()),
            email: sender_details.email,
            phone: sender_details.phone.as_ref().map(|p| p.number.clone().into()),
            phone_country_code: sender_details.phone_country_code,
        });
    }
    
    Ok(payment_request)
}

/// Transform remittance to payout request
pub fn remittance_to_payout_request(
    remittance: &storage::Remittance,
) -> CustomResult<payout_types::PayoutCreateRequest, errors::ApiErrorResponse> {
    // Parse beneficiary details
    let beneficiary_details: api::remittances::BeneficiaryDetails = remittance
        .beneficiary_details
        .parse_value("BeneficiaryDetails")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    
    // Map payout method from beneficiary details
    let payout_method_data = beneficiary_details
        .payout_details
        .and_then(|details| map_to_payout_method(details).ok());
    
    let payout_request = payout_types::PayoutCreateRequest {
        amount: Some(api::Amount {
            value: remittance.destination_amount.unwrap_or(remittance.amount),
            currency: remittance.destination_currency.parse().unwrap_or_default(),
        }),
        payout_method_data,
        customer_id: beneficiary_details.customer_id,
        name: Some(beneficiary_details.name.into()),
        email: beneficiary_details.email,
        phone: beneficiary_details.phone.as_ref().map(|p| p.number.clone().into()),
        phone_country_code: beneficiary_details.phone_country_code,
        address: beneficiary_details.address,
        confirm: Some(true),
        statement_descriptor: Some("REMITTANCE".to_string()),
        metadata: Some(json!({
            "remittance_id": remittance.id.to_string(),
            "type": "remittance_payout",
            "reference": remittance.reference,
        })),
        ..Default::default()
    };
    
    Ok(payout_request)
}

/// Map remittance payout method to API payout method
fn map_to_payout_method(
    payout_details: api::remittances::PayoutMethodData,
) -> CustomResult<payout_types::PayoutMethodData, errors::ApiErrorResponse> {
    match payout_details {
        api::remittances::PayoutMethodData::BankTransfer(bank_data) => {
            Ok(payout_types::PayoutMethodData::Bank(payout_types::Bank {
                account_number: bank_data.account_number,
                routing_number: bank_data.routing_number,
                bic: bank_data.bic,
                iban: bank_data.iban,
                bank_name: bank_data.bank_name,
                bank_country: bank_data.bank_country,
                // Map other fields as needed
                ..Default::default()
            }))
        }
        
        api::remittances::PayoutMethodData::Card(card_data) => {
            Ok(payout_types::PayoutMethodData::Card(payout_types::CardPayout {
                card_token: Some(card_data.card_token),
                // Map other fields as needed
                ..Default::default()
            }))
        }
        
        api::remittances::PayoutMethodData::Wallet(wallet_data) => {
            Ok(payout_types::PayoutMethodData::Wallet(payout_types::Wallet {
                wallet_type: Some(wallet_data.wallet_type.to_string()),
                wallet_id: Some(wallet_data.wallet_id),
                // Map other fields as needed
                ..Default::default()
            }))
        }
        
        api::remittances::PayoutMethodData::CashPickup(_) => {
            // Cash pickup might not be directly supported in payouts
            // Would need custom implementation or mapping
            Err(report!(errors::ApiErrorResponse::PayoutMethodNotSupported))
        }
    }
}