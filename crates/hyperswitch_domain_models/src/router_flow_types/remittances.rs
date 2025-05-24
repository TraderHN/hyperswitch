use api_models::{
    enums::{Currency, RemittanceStatus, FutureUsage, CountryAlpha2},
    payments::{Address, PaymentMethodData},
    remittances::{
        BeneficiaryDetails, ComplianceStatus, ExchangeRateInfo,
        RemittancePurpose, RequiredDocument, SenderDetails,
    },
};
use common_utils::{
    id_type::{MerchantId, PaymentId, ProfileId},
    pii::SecretSerdeValue,
    types::MinorUnit,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// Definir PayoutId localmente si no existe en common_utils
// O usar String directamente hasta que se implemente PayoutId
type PayoutId = String;

/// Request to create a new remittance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceCreateRequest {
    /// Optional unique identifier for idempotency (auto-generated if omitted)
    pub remittance_id: Option<String>,
    /// Optional merchant ID (inferred from API key if omitted)
    pub merchant_id: Option<MerchantId>,
    /// Optional profile ID to use
    pub profile_id: Option<ProfileId>,
    /// Optional connector to use (e.g. "wise", "currencycloud")
    pub connector: Option<String>,
    /// Amount in minor units (e.g. cents for USD)
    pub amount: MinorUnit,
    /// Source currency (in sender's country)
    pub source_currency: Currency,
    /// Destination currency (in beneficiary's country)
    pub destination_currency: Currency,
    /// Sender details
    pub sender_details: SenderDetails,
    /// Beneficiary details
    pub beneficiary_details: BeneficiaryDetails,
    /// Remittance date in YYYY-MM-DD format
    pub remittance_date: String,
    /// Reference/purpose for the remittance
    pub reference: String,
    /// Purpose code for the remittance
    pub purpose: Option<RemittancePurpose>,
    /// URL to redirect customer after payment
    pub return_url: Option<String>,
    /// Arbitrary metadata as key-value pairs
    pub metadata: Option<SecretSerdeValue>,
    /// Whether to automatically process payment and payout
    pub auto_process: Option<bool>,
    /// Optional routing algorithm configuration
    pub routing_algorithm: Option<serde_json::Value>,
    /// Optional custom connector credentials
    pub merchant_connector_details: Option<serde_json::Value>,
}

/// Request to pay for a remittance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemittancePayRequest {
    /// Payment method data (request)
    pub payment_method_data: Option<PaymentMethodData>,
    /// Billing address for the payment
    pub billing: Option<Address>,
    /// Immediate confirmation
    pub confirm: Option<bool>,
    /// Return URL
    pub return_url: Option<String>,
    /// Client secret
    pub client_secret: Option<String>,
    /// Save payment method for future use
    pub setup_future_usage: Option<FutureUsage>,
    /// Browser info for 3DS
    pub browser_info: Option<serde_json::Value>,
}

/// Request to update a remittance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceUpdateRequest {
    /// Updated reference
    pub reference: Option<String>,
    /// Updated metadata
    pub metadata: Option<SecretSerdeValue>,
    /// Updated beneficiary details
    pub beneficiary_details: Option<BeneficiaryDetails>,
    /// Updated return URL
    pub return_url: Option<String>,
}

/// Request to retrieve a remittance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceRetrieveRequest {
    /// Force sync with connector
    pub force_sync: Option<bool>,
    /// Client secret for authenticated access
    pub client_secret: Option<String>,
}

/// Request to list remittances with filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceListRequest {
    /// Optional status filters
    pub status: Option<Vec<RemittanceStatus>>,
    /// Optional connector filter
    pub connector: Option<String>,
    /// Optional source currency filter
    pub source_currency: Option<Currency>,
    /// Optional destination currency filter
    pub destination_currency: Option<Currency>,
    /// Optional time range filter
    pub time_range: Option<common_utils::types::TimeRange>,
    /// Pagination limit
    pub limit: Option<u32>,
    /// Pagination offset
    pub offset: Option<u32>,
}

/// Response to create/update/retrieve remittance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceResponse {
    /// Unique remittance identifier
    pub remittance_id: String,
    /// Merchant ID
    pub merchant_id: MerchantId,
    /// Profile ID
    pub profile_id: ProfileId,
    /// Amount in minor units
    pub amount: MinorUnit,
    /// Source currency
    pub source_currency: Currency,
    /// Destination currency
    pub destination_currency: Currency,
    /// Source amount in minor units
    pub source_amount: MinorUnit,
    /// Destination amount in minor units
    pub destination_amount: MinorUnit,
    /// Exchange rate information
    pub exchange_rate: Option<ExchangeRateInfo>,
    /// Sender details
    pub sender_details: Option<SenderDetails>,
    /// Beneficiary details
    pub beneficiary_details: Option<BeneficiaryDetails>,
    /// Remittance date YYYY-MM-DD
    pub remittance_date: String,
    /// Reference string
    pub reference: String,
    /// Purpose code
    pub purpose: Option<RemittancePurpose>,
    /// Current status
    pub status: RemittanceStatus,
    /// Failure reason if failed
    pub failure_reason: Option<String>,
    /// Return URL
    pub return_url: Option<String>,
    /// Metadata
    pub metadata: Option<SecretSerdeValue>,
    /// Connector used
    pub connector: String,
    /// Client secret for UI authentication
    pub client_secret: Option<String>,
    /// Payment ID associated with this remittance
    pub payment_id: Option<PaymentId>,
    /// Payout ID associated with this remittance
    pub payout_id: Option<PayoutId>,
    /// Connector transaction ID for payment
    pub payment_connector_transaction_id: Option<String>,
    /// Connector transaction ID for payout
    pub payout_connector_transaction_id: Option<String>,
    /// Compliance verification status
    pub compliance_status: Option<ComplianceStatus>,
    /// Required documents for compliance
    pub required_documents: Option<Vec<RequiredDocument>>,
    /// Estimated delivery time
    pub estimated_delivery_time: Option<String>,
    /// Actual delivery time
    pub actual_delivery_time: Option<String>,
    /// Creation timestamp
    pub created_at: Option<time::PrimitiveDateTime>,
    /// Last update timestamp
    pub updated_at: Option<time::PrimitiveDateTime>,
}

/// List response for remittances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceListResponse {
    /// Total number of items matching criteria
    pub total_count: i64,
    /// Number of items in this response
    pub count: usize,
    /// Remittance data
    pub data: Vec<RemittanceResponse>,
}

/// Manual status update for remittance (admin-only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceManualUpdateRequest {
    /// Remittance ID to update
    pub remittance_id: String,
    /// Merchant ID
    pub merchant_id: MerchantId,
    /// New status
    pub status: RemittanceStatus,
    /// Optional error code
    pub error_code: Option<String>,
    /// Optional error message
    pub error_message: Option<String>,
    /// Optional error reason
    pub error_reason: Option<String>,
    /// Optional connector transaction ID
    pub connector_transaction_id: Option<String>,
}

/// Request to sync remittance status 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceSyncRequest {
    /// Merchant ID to sync remittances for
    pub merchant_id: MerchantId,
    /// Optional time range to limit remittances
    pub time_range: Option<common_utils::types::TimeRange>,
    /// Force sync even if recently synced
    pub force_sync: Option<bool>,
}

/// Response for remittance sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceSyncResponse {
    /// Merchant ID that was synced
    pub merchant_id: MerchantId,
    /// Number of remittances synced
    pub synced_count: usize,
    /// Individual sync results
    pub results: Vec<RemittanceSyncResult>,
}

/// Individual sync result for a remittance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceSyncResult {
    /// Remittance ID
    pub remittance_id: String,
    /// Previous status
    pub previous_status: RemittanceStatus,
    /// Current status after sync
    pub current_status: RemittanceStatus,
    /// Sync timestamp
    pub synced_at: OffsetDateTime,
    /// Whether payment data was updated
    pub payment_updated: bool,
    /// Whether payout data was updated
    pub payout_updated: bool,
}

/// Request for getting a remittance exchange rate quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceQuoteRequest {
    /// Source currency
    pub source_currency: Currency,
    /// Destination currency
    pub destination_currency: Currency,
    /// Amount in minor units
    pub amount: MinorUnit,
    /// Optional connector to get quote from
    pub connector: Option<String>,
    /// Sender country
    pub source_country: Option<CountryAlpha2>,
    /// Beneficiary country
    pub destination_country: Option<CountryAlpha2>,
}

/// Response for remittance exchange rate quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceQuoteResponse {
    /// Source currency
    pub source_currency: Currency,
    /// Destination currency
    pub destination_currency: Currency,
    /// Source amount
    pub source_amount: MinorUnit,
    /// Calculated destination amount
    pub destination_amount: MinorUnit,
    /// Exchange rate (1 source = X destination)
    pub rate: f64,
    /// Fee in source currency
    pub fee: Option<MinorUnit>,
    /// Total cost including fees
    pub total_cost: MinorUnit,
    /// Estimated delivery time (hours)
    pub estimated_delivery_time: Option<u32>,
    /// Rate expiry timestamp
    pub rate_valid_until: Option<time::PrimitiveDateTime>,
    /// Connector providing the rate
    pub connector: String,
}