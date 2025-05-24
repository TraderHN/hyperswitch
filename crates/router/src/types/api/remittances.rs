//! API types for remittances

pub use api_models::remittances::{
    BankAccountType, BankTransferData, BeneficiaryDetails, CashPickupData, CardPayoutData,
    ComplianceCheckStatus, ComplianceStatus, CustomerDetails, DocumentStatus, ExchangeRateInfo,
    PayoutMethodData, RemittanceCreateRequest, RemittanceListRequest, RemittanceListResponse,
    RemittanceManualUpdateRequest, RemittancePayRequest, RemittancePurpose, RemittanceQuoteRequest,
    RemittanceQuoteResponse, RemittanceRetrieveRequest, RemittanceResponse, RemittanceStatus,
    RemittanceSyncRequest, RemittanceSyncResponse, RemittanceSyncResult, RemittanceUpdateRequest,
    RequiredDocument, SenderDetails, WalletPayoutData, WalletType,
};

use serde::{Deserialize, Serialize};

/// Remittance operations for connector integration
#[derive(Debug, Clone)]
pub struct RemittanceQuote;

#[derive(Debug, Clone)]
pub struct RemittanceCreate;

#[derive(Debug, Clone)]
pub struct RemittanceStatus;

#[derive(Debug, Clone)]
pub struct RemittanceCancel;

#[derive(Debug, Clone)]
pub struct RemittancePayout;

#[derive(Debug, Clone)]
pub struct RemittanceExecute;

/// Request data types for remittance operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum RemittanceExecuteRequestData {
    Quote(RemittanceQuoteRequestData),
    Create(RemittanceCreateRequestData),
    Status(RemittanceStatusRequestData),
    Cancel(RemittanceCancelRequestData),
    Payout(RemittancePayoutRequestData),
}

/// Response data types for remittance operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum RemittanceExecuteResponseData {
    Quote(RemittanceQuoteResponseData),
    Create(RemittanceCreateResponseData),
    Status(RemittanceStatusResponseData),
    Cancel(RemittanceCancelResponseData),
    Payout(RemittancePayoutResponseData),
}

impl Default for RemittanceExecuteResponseData {
    fn default() -> Self {
        Self::Status(RemittanceStatusResponseData::default())
    }
}

/// Quote request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceQuoteRequestData {
    pub source_currency: api_models::enums::Currency,
    pub destination_currency: api_models::enums::Currency,
    pub source_amount: i64,
    pub source_country: Option<api_models::enums::CountryAlpha2>,
    pub destination_country: Option<api_models::enums::CountryAlpha2>,
}

/// Quote response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceQuoteResponseData {
    pub source_currency: api_models::enums::Currency,
    pub destination_currency: api_models::enums::Currency,
    pub source_amount: i64,
    pub destination_amount: i64,
    pub exchange_rate: f64,
    pub fee: Option<i64>,
    pub estimated_delivery_hours: Option<u32>,
    pub rate_expiry_time: Option<time::PrimitiveDateTime>,
    pub quote_id: Option<String>,
}

/// Create request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceCreateRequestData {
    pub remittance_id: String,
    pub source_currency: api_models::enums::Currency,
    pub destination_currency: api_models::enums::Currency,
    pub source_amount: i64,
    pub destination_amount: Option<i64>,
    pub sender: RemittanceSenderData,
    pub beneficiary: RemittanceBeneficiaryData,
    pub purpose: Option<String>,
    pub reference: String,
    pub metadata: Option<common_utils::pii::SecretSerdeValue>,
}

/// Create response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceCreateResponseData {
    pub connector_remittance_id: String,
    pub status: RemittanceConnectorStatus,
    pub created_at: Option<time::PrimitiveDateTime>,
    pub estimated_delivery_time: Option<time::PrimitiveDateTime>,
    pub exchange_rate: Option<f64>,
    pub fee: Option<i64>,
    pub destination_amount: Option<i64>,
}

/// Status request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceStatusRequestData {
    pub remittance_id: String,
    pub connector_remittance_id: Option<String>,
}

/// Status response data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemittanceStatusResponseData {
    pub status: RemittanceConnectorStatus,
    pub payment_status: Option<PaymentStageStatus>,
    pub payout_status: Option<PayoutStageStatus>,
    pub failure_reason: Option<String>,
    pub updated_at: Option<time::PrimitiveDateTime>,
    pub completed_at: Option<time::PrimitiveDateTime>,
}

/// Cancel request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceCancelRequestData {
    pub remittance_id: String,
    pub connector_remittance_id: Option<String>,
    pub reason: String,
}

/// Cancel response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceCancelResponseData {
    pub status: RemittanceConnectorStatus,
    pub cancelled_at: Option<time::PrimitiveDateTime>,
    pub refund_status: Option<RefundStatus>,
}

/// Payout request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittancePayoutRequestData {
    pub remittance_id: String,
    pub amount: i64,
    pub currency: api_models::enums::Currency,
    pub beneficiary: RemittanceBeneficiaryData,
}

/// Payout response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittancePayoutResponseData {
    pub payout_id: String,
    pub status: PayoutStageStatus,
    pub created_at: Option<time::PrimitiveDateTime>,
}

/// Sender data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceSenderData {
    pub name: String,
    pub address: Option<api_models::payments::Address>,
    pub email: Option<common_utils::pii::Email>,
    pub phone: Option<api_models::payments::PhoneDetails>,
}

/// Beneficiary data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceBeneficiaryData {
    pub name: String,
    pub address: Option<api_models::payments::Address>,
    pub email: Option<common_utils::pii::Email>,
    pub phone: Option<api_models::payments::PhoneDetails>,
    pub account_details: RemittanceAccountDetails,
}

/// Account details for remittance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemittanceAccountDetails {
    Bank {
        account_number: masking::Secret<String>,
        routing_number: Option<masking::Secret<String>>,
        iban: Option<masking::Secret<String>>,
        bic: Option<masking::Secret<String>>,
        bank_name: Option<String>,
        bank_country: Option<String>,
    },
    Wallet {
        wallet_id: String,
        wallet_type: String,
        provider_details: Option<common_utils::pii::SecretSerdeValue>,
    },
    Other,
}

/// Connector status for remittance
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum RemittanceConnectorStatus {
    #[default]
    Created,
    Processing,
    RequiresAction,
    Completed,
    Failed,
    Cancelled,
}

/// Payment stage status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentStageStatus {
    NotStarted,
    Initiated,
    Processing,
    Authorized,
    Captured,
    Failed,
}

/// Payout stage status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayoutStageStatus {
    NotStarted,
    Initiated,
    Processing,
    Sent,
    Delivered,
    Failed,
}

/// Refund status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefundStatus {
    NotStarted,
    Initiated,
    Processing,
    Completed,
    Failed,
}