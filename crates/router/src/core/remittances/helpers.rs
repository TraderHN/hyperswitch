//! Helper functions for remittances

use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::{
    core::errors,
    db::StorageInterface,
    types::{domain, storage},
    SessionState,
};

/// Get exchange rate from connector or fallback service
pub async fn get_exchange_rate(
    state: &SessionState,
    source_currency: &api_models::enums::Currency,
    destination_currency: &api_models::enums::Currency,
    amount: i64,
    connector: Option<&String>,
) -> CustomResult<Decimal, errors::ApiErrorResponse> {
    // TODO: Call actual connector or exchange rate service
    // For now, return mock rate
    
    let rate = match (source_currency, destination_currency) {
        (api_models::enums::Currency::USD, api_models::enums::Currency::EUR) => Decimal::from_str("0.85")?,
        (api_models::enums::Currency::USD, api_models::enums::Currency::GBP) => Decimal::from_str("0.73")?,
        (api_models::enums::Currency::EUR, api_models::enums::Currency::USD) => Decimal::from_str("1.18")?,
        (api_models::enums::Currency::GBP, api_models::enums::Currency::USD) => Decimal::from_str("1.37")?,
        _ => Decimal::from_str("1.0")?, // Default 1:1
    };
    
    Ok(rate)
}

/// Get exchange rate with fee
pub async fn get_exchange_rate_with_fee(
    state: &SessionState,
    source_currency: &api_models::enums::Currency,
    destination_currency: &api_models::enums::Currency,
    amount: i64,
    connector: Option<&String>,
) -> CustomResult<(Decimal, Option<i64>), errors::ApiErrorResponse> {
    let rate = get_exchange_rate(state, source_currency, destination_currency, amount, connector).await?;
    
    // Calculate fee (2% for example)
    let fee = (amount as f64 * 0.02) as i64;
    
    Ok((rate, Some(fee)))
}

/// Calculate destination amount based on exchange rate
pub fn calculate_destination_amount(source_amount: i64, exchange_rate: Decimal) -> i64 {
    let result = Decimal::from(source_amount) * exchange_rate;
    result.round().to_i64().unwrap_or(0)
}

/// Find remittance by connector reference
pub async fn find_remittance_by_connector_reference(
    db: &dyn StorageInterface,
    merchant_id: &str,
    reference_id: &str,
) -> CustomResult<storage::Remittance, errors::ApiErrorResponse> {
    // TODO: Implement actual lookup by connector reference
    // This would typically search by payment_id or payout_id in the respective tables
    
    Err(report!(errors::ApiErrorResponse::RemittanceNotFound))
}

/// Webhook data structure
#[derive(Debug, Clone)]
pub struct WebhookData {
    pub reference_id: String,
    pub status: String,
    pub webhook_type: WebhookType,
    pub connector_reference: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum WebhookType {
    Payment,
    Payout,
}

/// Parse connector webhook
pub fn parse_connector_webhook(
    connector_name: &str,
    body: &serde_json::Value,
) -> CustomResult<WebhookData, errors::ApiErrorResponse> {
    // TODO: Implement connector-specific webhook parsing
    // This is a placeholder implementation
    
    match connector_name {
        "stripe" => parse_stripe_webhook(body),
        "wise" => parse_wise_webhook(body),
        _ => Err(report!(errors::ApiErrorResponse::WebhookProcessingFailure)),
    }
}

fn parse_stripe_webhook(body: &serde_json::Value) -> CustomResult<WebhookData, errors::ApiErrorResponse> {
    // Placeholder for Stripe webhook parsing
    Ok(WebhookData {
        reference_id: body["data"]["object"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        status: body["data"]["object"]["status"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        webhook_type: WebhookType::Payment,
        connector_reference: body["data"]["object"]["id"].as_str().map(String::from),
        error_code: None,
        error_message: None,
    })
}

fn parse_wise_webhook(body: &serde_json::Value) -> CustomResult<WebhookData, errors::ApiErrorResponse> {
    // Placeholder for Wise webhook parsing
    Ok(WebhookData {
        reference_id: body["resource"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        status: body["event_type"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        webhook_type: WebhookType::Payout,
        connector_reference: body["resource"]["id"].as_str().map(String::from),
        error_code: None,
        error_message: None,
    })
}

/// Generate client secret for remittance
pub fn generate_remittance_client_secret(remittance_id: &uuid::Uuid) -> String {
    format!("rem_{}_{}", remittance_id, uuid::Uuid::new_v4())
}

/// Mask sensitive data for logging
pub fn mask_sensitive_data(data: &str) -> String {
    if data.len() > 8 {
        format!("{}****{}", &data[..4], &data[data.len()-4..])
    } else {
        "****".to_string()
    }
}