// ----------------------
// Dependencias externas
// ----------------------
use common_utils::id_type::{CustomerId, MerchantId, PaymentId, ProfileId};
use common_utils::pii::{Email, SecretSerdeValue};
use common_utils::types::{MinorUnit, TimeRange, };          //  ←  Url sólo una vez
use masking::Secret;
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::{OffsetDateTime, PrimitiveDateTime};                  //  ←  añadimos OffsetDateTime
use utoipa::ToSchema;

// ----------------------
// Módulos internos
// ----------------------
use crate::admin::MerchantConnectorDetailsWrap;
use crate::enums::{CardNetwork, CountryAlpha2, Currency, FutureUsage, PayoutType};

use url::Url as ExternalUrl;

#[cfg(feature = "v1")]
use crate::payments::{
    Address, Amount, CustomerDetails as PaymentsCustomerDetails,
    PaymentMethodDataRequest, PaymentsRequest, PhoneDetails,
};

#[cfg(feature = "v2")]
use crate::payments::{
    Address, Amount, AmountDetails, PaymentMethodDataRequest,
    PaymentsRequest, PhoneDetails,
};

use crate::payouts::PayoutCreateRequest;


// ===== enums =====

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Deserialize, Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum RemittancePurpose {
    FamilySupport,
    Education,
    Medical,
    Business,
    Gift,
    Donation,
    LoanRepayment,
    Salary,
    PropertyPurchase,
    Utility,
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToSchema, Deserialize, Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum WalletType {
    /// Mobile money wallet
    MobileMoney,
    /// Digital wallet
    DigitalWallet,
    /// Bank wallet
    BankWallet,
    /// Crypto wallet
    CryptoWallet,
    /// Other wallet type
    Other,
}

// ============================================
// REQUEST MODELS
// ============================================

/// Create a new remittance
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemittanceRequest {
    /// Optional unique identifier for idempotency (auto-generated if omitted)
    #[schema(
        max_length = 36,
        min_length = 32,
        example = "rem_a6b8e3f41234567891234abcdefabcdef",
        value_type = Option<String>
    )]
    pub remittance_id: Option<String>,

    /// Optional merchant ID (inferred from API key if omitted)
    #[schema(
        max_length = 255,
        example = "merchant_1668273825",
        value_type = Option<String>
    )]
    pub merchant_id: Option<MerchantId>,

    /// Optional profile ID to use
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<ProfileId>,

    /// Optional connector to use (e.g. "wise", "currencycloud")
    #[schema(example = "wise", value_type = Option<String>)]
    pub connector: Option<String>,

    /// Amount in minor units (e.g. cents for USD)
    #[schema(minimum = 1, example = 1000, value_type = i64)]
    pub amount: MinorUnit,

    /// Source currency (in sender's country)
    #[schema(value_type = Currency, example = "USD")]
    pub source_currency: Currency,

    /// Destination currency (in beneficiary's country)
    #[schema(value_type = Currency, example = "MXN")]
    pub destination_currency: Currency,

    /// Sender details
    pub sender_details: SenderDetails,

    /// Beneficiary details
    pub beneficiary_details: BeneficiaryDetails,

    /// Remittance date in YYYY-MM-DD format
    #[schema(
        pattern = "^\\d{4}-\\d{2}-\\d{2}$",
        example = "2023-06-15",
        value_type = String
    )]
    pub remittance_date: String,

    /// Reference/purpose for the remittance
    #[schema(
        max_length = 100,
        min_length = 1,
        example = "Family support",
        value_type = String
    )]
    pub reference: String,

    /// Purpose code for the remittance
    #[schema(value_type = Option<RemittancePurpose>, example = "family_support")]
    pub purpose: Option<RemittancePurpose>,

    /// URL to redirect customer after payment
    #[schema(format = "uri", max_length = 500, example = "https://merchant.example.com/callback")]
    pub return_url: Option<String>,

    /// Arbitrary metadata as key-value pairs
    #[schema(value_type = Option<Object>, example = r#"{ "note": "Urgent", "invoice_id": "INV-123" }"#)]
    pub metadata: Option<SecretSerdeValue>,

    /// Whether to automatically process payment and payout
    #[schema(default = true, example = true)]
    pub auto_process: Option<bool>,

    /// Optional routing algorithm configuration
    pub routing_algorithm: Option<serde_json::Value>,

    /// Optional custom connector credentials
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<MerchantConnectorDetailsWrap>,
}

impl Default for RemittanceRequest {
    fn default() -> Self {
        Self {
            remittance_id: None,
            merchant_id:   None,
            profile_id:    None,
            connector:     None,
            amount:        MinorUnit::new(0),
            source_currency:      Currency::USD,
            destination_currency: Currency::USD,
            sender_details:       SenderDetails::default(),
            beneficiary_details:  BeneficiaryDetails::default(),
            // OffsetDateTime::now_utc() ya está disponible tras el import
            remittance_date: OffsetDateTime::now_utc().date().to_string(),
            reference: String::new(),
            purpose:   None,
            return_url: None,
            metadata:   None,
            auto_process: Some(true),
            routing_algorithm: None,
            merchant_connector_details: None,
        }
    }
}


/// Fund a pending remittance with a payment
#[derive(Debug, ToSchema, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemittancePayRequest {
    /// Payment method data (request)
    pub payment_method_data: Option<PaymentMethodDataRequest>,

    /// Billing address for the payment
    pub billing: Option<Address>,

    /// Immediate confirmation
    #[schema(default = true)]
    pub confirm: Option<bool>,

    /// Return URL
    pub return_url: Option<String>,

    /// Client secret
    pub client_secret: Option<String>,

    /// Save payment method for future use
    #[schema(value_type = Option<FutureUsage>)]
    pub setup_future_usage: Option<FutureUsage>,
}

impl RemittancePayRequest {
    #[cfg(feature = "v1")]
    pub fn to_payment_request(
        &self,
        remittance_id: &str,
        amount: MinorUnit,
        currency: Currency,
        reference: &str,
        sender_name: &str,
        return_url: Option<String>,
    ) -> PaymentsRequest {
        let metadata = Some(json!({          // ← sin Secret en v1
            "remittance_id": remittance_id,
            "type":          "remittance_payment",
            "reference":     reference,
        }));
    
        let mut req = PaymentsRequest {
            amount:              Some(Amount::from(amount)),
            currency:            Some(currency),
            payment_method_data: self.payment_method_data.clone(),
            billing:             self.billing.clone(),
            confirm:             self.confirm,
            return_url:          self.return_url.clone()
                                   .or(return_url)
                                   .and_then(|u| ExternalUrl::parse(&u).ok()),
            client_secret:       self.client_secret.clone(),
            setup_future_usage:  self.setup_future_usage,
            description:         Some(format!("Remittance: {reference}")),
            metadata,                                // ← ahora Value
            ..Default::default()
        };
    
        // customer
        req.customer = Some(PaymentsCustomerDetails {
            id:  CustomerId::default(),
            name: Some(Secret::new(sender_name.to_owned())),
            email: None,
            phone: None,
            phone_country_code: None,
        });
    
        req
    }
    
    #[cfg(feature = "v2")]
    pub fn to_payment_request(
        &self,
        remittance_id: &str,
        amount: MinorUnit,
        currency: Currency,
        reference: &str,
        sender_name: &str,
        return_url: Option<String>,
    ) -> PaymentsRequest {
        // 1) Detalle de montos
        let amount_details = AmountDetails {
            order_amount:  Amount::from(amount),
            currency,
            shipping_cost: None,
            order_tax_amount: None,
            skip_external_tax_calculation: crate::enums::TaxCalculationOverride::Skip,
            skip_surcharge_calculation:    crate::enums::SurchargeCalculationOverride::Skip,
            surcharge_amount: None,
            tax_on_surcharge: None,
        };
    
    // el tipo que espera `PaymentsRequest` es Option<SecretSerdeValue>
    let metadata: Option<SecretSerdeValue> = Some(Secret::new(json!({
        "remittance_id": remittance_id,
        "type":          "remittance_payment",
        "reference":     reference,
    })));
    
    let parsed_return_url = self.return_url
    .clone()
    .or(return_url)
    .and_then(|u| ExternalUrl::parse(&u).ok())
    .map(Url::wrap);                // convierte url::Url → common_utils::types::Url

    
        PaymentsRequest {
            amount_details,
            description: Some(common_utils::types::Description(format!("Remittance: {}", reference))),
            return_url:  parsed_return_url,
            metadata,
            billing:     self.billing.clone(),
            payment_method_data: self.payment_method_data.clone(),
            setup_future_usage:  self.setup_future_usage,
            // ─── todo lo demás queda Default ───
            ..Default::default()
        }
    }
}
/// Update a remittance
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemittanceUpdateRequest {
    /// Updated reference
    #[schema(
        max_length = 100,
        min_length = 1,
        example = "Family support - updated",
        value_type = Option<String>
    )]
    pub reference: Option<String>,
    
    /// Updated metadata
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<SecretSerdeValue>,
    
    /// Updated beneficiary details
    pub beneficiary_details: Option<BeneficiaryDetails>,
    
    /// Updated return URL
    pub return_url: Option<String>,
}

/// Retrieve a remittance
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemittancesRetrieveRequest {
    /// Force sync with connector
    #[schema(default = false)]
    pub force_sync: Option<bool>,
    
    /// Client secret for authenticated access
    pub client_secret: Option<String>,
}

/// List remittances with filters
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemittanceListRequest {
    /// Optional status filters
    pub status: Option<Vec<RemittanceStatus>>,
    
    /// Optional connector filter
    pub connector: Option<String>,
    
    /// Optional source currency filter
    #[schema(value_type = Option<Currency>)]
    pub source_currency: Option<Currency>,
    
    /// Optional destination currency filter
    #[schema(value_type = Option<Currency>)]
    pub destination_currency: Option<Currency>,
    
    /// Optional time range filter
    pub time_range: Option<TimeRange>,
    
    /// Pagination limit
    #[schema(default = 10, maximum = 100, minimum = 1)]
    pub limit: Option<u32>,
    
    /// Pagination offset
    #[schema(default = 0, minimum = 0)]
    pub offset: Option<u32>,
}

/// Manual status update for remittance (admin-only)
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemittanceManualUpdateRequest {
    /// Remittance ID to update
    #[schema(example = "rem_a6b8e3f41234567891234abcdefabcdef")]
    pub remittance_id: String,
    
    /// Merchant ID
    #[schema(example = "merchant_1668273825")]
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

/// Sync status for multiple remittances
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RemittanceSyncRequest {
    /// Merchant ID to sync remittances for
    pub merchant_id: MerchantId,
    
    /// Optional time range to limit remittances
    pub time_range: Option<TimeRange>,
    
    /// Force sync even if recently synced
    pub force_sync: Option<bool>,
}

/// Request exchange rate quote
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RemittanceQuoteRequest {
    /// Source currency
    #[schema(value_type = Currency)]
    pub source_currency: Currency,
    
    /// Destination currency
    #[schema(value_type = Currency)]
    pub destination_currency: Currency,
    
    /// Amount in minor units
    pub amount: MinorUnit,
    
    /// Optional connector to get quote from
    pub connector: Option<String>,
    
    /// Sender country
    #[schema(value_type = Option<CountryAlpha2>)]
    pub source_country: Option<CountryAlpha2>,
    
    /// Beneficiary country
    #[schema(value_type = Option<CountryAlpha2>)]
    pub destination_country: Option<CountryAlpha2>,
}

// ============================================
// RESPONSE MODELS
// ============================================

/// Remittance response 
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RemittanceResponse {
    /// Unique remittance identifier
    pub remittance_id: String,
    
    /// Merchant ID
    #[schema(value_type = String)]
    pub merchant_id: MerchantId,
    
    /// Profile ID
    #[schema(value_type = String)]
    pub profile_id: ProfileId,
    
    /// Amount in minor units
    #[schema(value_type = i64, minimum = 0)]
    pub amount: MinorUnit,
    
    /// Source currency
    #[schema(value_type = Currency)]
    pub source_currency: Currency,
    
    /// Destination currency
    #[schema(value_type = Currency)]
    pub destination_currency: Currency,
    
    /// Source amount in minor units
    #[schema(value_type = i64, minimum = 0)]
    pub source_amount: MinorUnit,
    
    /// Destination amount in minor units
    #[schema(value_type = i64, minimum = 0)]
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
    #[schema(value_type = Option<RemittancePurpose>)]
    pub purpose: Option<RemittancePurpose>,
    
    /// Current status
    pub status: RemittanceStatus,
    
    /// Failure reason if failed
    pub failure_reason: Option<String>,
    
    /// Return URL
    pub return_url: Option<String>,
    
    /// Metadata
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<SecretSerdeValue>,
    
    /// Connector used
    #[schema(example = "wise")]
    pub connector: String,
    
    /// Client secret for UI authentication
    pub client_secret: Option<String>,
    
    /// Payment ID associated with this remittance
    #[schema(value_type = Option<String>)]
    pub payment_id: Option<PaymentId>,
    
    /// Payout ID associated with this remittance
    #[schema(value_type = Option<String>)]
    pub payout_id: Option<String>,
    
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
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    
    /// Last update timestamp
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

/// List response with pagination
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RemittanceListResponse {
    /// Total number of items matching criteria
    pub total_count: i64,
    
    /// Number of items in this response
    pub count: usize,
    
    /// Remittance data
    pub data: Vec<RemittanceResponse>,
}

/// Sync response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RemittanceSyncResponse {
    /// Merchant ID that was synced
    #[schema(value_type = String)]
    pub merchant_id: MerchantId,
    
    /// Number of remittances synced
    pub synced_count: usize,
    
    /// Individual sync results
    pub results: Vec<RemittanceSyncResult>,
}

/// Individual sync result
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RemittanceSyncResult {
    /// Remittance ID
    pub remittance_id: String,
    
    /// Previous status
    pub previous_status: RemittanceStatus,
    
    /// Current status after sync
    pub current_status: RemittanceStatus,
    
    /// Sync timestamp
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub synced_at: PrimitiveDateTime,
    
    /// Whether payment data was updated
    pub payment_updated: bool,
    
    /// Whether payout data was updated
    pub payout_updated: bool,
}

/// Exchange rate quote response
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RemittanceQuoteResponse {
    /// Source currency
    #[schema(value_type = Currency)]
    pub source_currency: Currency,
    
    /// Destination currency
    #[schema(value_type = Currency)]
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
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub rate_valid_until: Option<PrimitiveDateTime>,
    
    /// Connector providing the rate
    pub connector: String,
}

// ============================================
// SUPPORTING MODELS
// ============================================

/// Sender details
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default)]
pub struct SenderDetails {
    /// Full name of sender
    #[schema(
        max_length = 255,
        min_length = 2,
        example = "Jane Smith",
        value_type = String
    )]
    pub name: String,
    
    /// Optional customer ID if sender is an existing customer
    #[schema(value_type = Option<String>, example = "cus_12345")]
    pub customer_id: Option<CustomerId>,
    
    /// Optional customer details
    pub customer_details: Option<CustomerDetails>,
    
    /// Optional email
    #[schema(value_type = Option<String>, example = "jane.smith@example.com")]
    pub email: Option<Email>,
    
    /// Optional phone number
    #[schema(value_type = Option<String>, example = "4155550123")]
    pub phone: Option<PhoneDetails>,
    
    /// Optional phone country code
    #[schema(value_type = Option<String>, example = "+1")]
    pub phone_country_code: Option<String>,
    
    /// Optional address
    pub address: Option<Address>,
    
    /// Payment method data to use for funding
    pub payment_method_data: Option<PaymentMethodDataRequest>,
}

/// Beneficiary details
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default)]
pub struct BeneficiaryDetails {
    /// Full name of beneficiary
    #[schema(
        max_length = 255,
        min_length = 2,
        example = "John Doe",
        value_type = String
    )]
    pub name: String,
    
    /// Optional first name
    #[schema(value_type = Option<String>, example = "John")]
    pub first_name: Option<String>,
    
    /// Optional last name
    #[schema(value_type = Option<String>, example = "Doe")]
    pub last_name: Option<String>,
    
    /// Optional customer ID
    #[schema(value_type = Option<String>, example = "cus_67890")]
    pub customer_id: Option<CustomerId>,
    
    /// Optional email
    #[schema(value_type = Option<String>, example = "john.doe@example.com")]
    pub email: Option<Email>,
    
    /// Optional phone number
    #[schema(value_type = Option<String>, example = "5255550199")]
    pub phone: Option<PhoneDetails>,
    
    /// Optional phone country code
    #[schema(value_type = Option<String>, example = "+52")]
    pub phone_country_code: Option<String>,
    
    /// Optional address
    pub address: Option<Address>,
    
    /// Payout method details (bank account, wallet, etc.)
    pub payout_details: Option<PayoutMethodData>,
    
    /// Optional relationship to sender
    #[schema(value_type = Option<String>, example = "Family")]
    pub relationship: Option<String>,
}

/// Customer details
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default)]
pub struct CustomerDetails {
    /// Full name
    #[schema(value_type = Option<String>)]
    pub name: Option<String>,
    
    /// Email
    #[schema(value_type = Option<String>)]
    pub email: Option<Email>,
    
    /// Phone number
    #[schema(value_type = Option<String>)]
    pub phone: Option<PhoneDetails>,
    
    /// Phone country code
    #[schema(value_type = Option<String>)]
    pub phone_country_code: Option<String>,
}

/// Exchange rate information
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ExchangeRateInfo {
    /// Exchange rate (1 source = X destination)
    pub rate: f64,
    
    /// Optional markup percentage
    pub markup: Option<f64>,
    
    /// Source currency
    #[schema(value_type = Currency)]
    pub source_currency: Currency,
    
    /// Destination currency
    #[schema(value_type = Currency)]
    pub destination_currency: Currency,
    
    /// Rate valid until timestamp
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub valid_until: Option<PrimitiveDateTime>,
}

/// Payout method data
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayoutMethodData {
    /// Bank transfer details
    BankTransfer(BankTransferData),
    
    /// Card payout details (push-to-card)
    Card(CardPayoutData),
    
    /// Digital wallet payout
    Wallet(WalletPayoutData),
    
    /// Cash pickup
    CashPickup(CashPickupData),
}

/// Bank account details
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BankTransferData {
    /// Account number
    #[schema(value_type = String)]
    pub account_number: Secret<String>,
    
    /// Routing/Sort/BSB code
    #[schema(value_type = Option<String>)]
    pub routing_number: Option<Secret<String>>,
    
    /// BIC/SWIFT code
    #[schema(value_type = Option<String>)]
    pub bic: Option<Secret<String>>,
    
    /// IBAN
    #[schema(value_type = Option<String>)]
    pub iban: Option<Secret<String>>,
    
    /// Bank name
    pub bank_name: Option<String>,
    
    /// Bank country
    #[schema(value_type = Option<CountryAlpha2>)]
    pub bank_country: Option<CountryAlpha2>,
    
    /// Bank address
    pub bank_address: Option<String>,
    
    /// Account type
    pub account_type: Option<BankAccountType>,
}

/// Bank account type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankAccountType {
    /// Checking account
    Checking,
    
    /// Savings account
    Savings,
    
    /// Corporate account
    Corporate,
}

/// Card payout data
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CardPayoutData {
    /// Card token for push-to-card
    pub card_token: String,
    
    /// Last four digits
    pub last4: Option<String>,
    
    /// Card network
    #[schema(value_type = Option<CardNetwork>)]
    pub card_network: Option<CardNetwork>,
}

/// Digital wallet data
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct WalletPayoutData {
    /// Wallet type
    #[schema(value_type = WalletType)]
    pub wallet_type: WalletType,
    
    /// Wallet ID/phone number
    pub wallet_id: String,
    
    /// Provider-specific data
    #[schema(value_type = Option<Object>)]
    pub provider_details: Option<SecretSerdeValue>,
}

/// Cash pickup data
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CashPickupData {
    /// Pickup location code
    pub location_code: String,
    
    /// Pickup location name
    pub location_name: Option<String>,
    
    /// Pickup country
    #[schema(value_type = Option<CountryAlpha2>)]
    pub country: Option<CountryAlpha2>,
    
    /// Additional pickup instructions
    pub instructions: Option<String>,
}

/// Compliance status
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ComplianceStatus {
    /// Current compliance status
    #[schema(value_type = ComplianceCheckStatus)]
    pub status: ComplianceCheckStatus,
    
    /// Required information list
    pub required_info: Option<Vec<String>>,
    
    /// Status message
    pub message: Option<String>,
}

/// Compliance check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceCheckStatus {
    /// Initial state
    Pending,
    
    /// Under manual review
    Reviewing,
    
    /// Approved
    Approved,
    
    /// Rejected
    Rejected,
    
    /// Additional information required
    AdditionalInfoRequired,
}

/// Required document
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RequiredDocument {
    /// Document type
    pub document_type: String,
    
    /// Description
    pub description: Option<String>,
    
    /// Document status
    pub status: DocumentStatus,
}

/// Document status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// Document required
    Required,
    
    /// Document submitted
    Submitted,
    
    /// Document approved
    Approved,
    
    /// Document rejected
    Rejected,
}

/// Remittance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, ToSchema, Deserialize, Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum RemittanceStatus {
    /// Initial state
    Created,
    
    /// Payment authorized but not captured
    Authorized,
    
    /// Payment captured, ready for payout
    Captured,
    
    /// Payout initiated
    PayoutInitiated,
    
    /// Completely fulfilled
    Completed,
    
    /// Failed
    Failed,
    
    /// Cancelled
    Cancelled,
    
    /// Reversed/refunded
    Reversed,
    
    /// Requires customer action (3DS, etc.)
    RequiresCustomerAction,
    
    /// Being processed
    Processing,
}

// ============================================
// IMPLEMENTATION HELPERS
// ============================================

impl PayoutMethodData {
    /// Get the method type
    pub fn get_method_type(&self) -> &'static str {
        match self {
            Self::BankTransfer(_) => "bank_transfer",
            Self::Card(_) => "card",
            Self::Wallet(_) => "wallet",
            Self::CashPickup(_) => "cash_pickup",
        }
    }
}
// ============================================
// HELPER: construir PayoutCreateRequest
// ============================================
#[cfg(any(feature = "v1", feature = "v2"))]
pub fn create_payout_request(
    remittance_id:      &str,
    amount:             MinorUnit,
    currency:           Currency,
    reference:          &str,
    beneficiary_name:   &str,
    beneficiary_email:  Option<Email>,
    payout_method_data: Option<&PayoutMethodData>,
) -> PayoutCreateRequest {
    // ---------- base --------------------------------------------------
    let mut req = PayoutCreateRequest {
        amount:      Some(Amount::from(amount)),
        currency:    Some(currency),
        description: Some(format!("Remittance payout: {reference}")),
        confirm:     Some(true),
        // metadata necesita SecretSerdeValue
        metadata: Some(Secret::new(json!({
            "remittance_id": remittance_id,
            "reference":     reference,
        }))),
        ..Default::default()
    };

    req.name  = Some(Secret::new(beneficiary_name.to_owned()));
    req.email = beneficiary_email;

    // ---------- sólo marcamos el tipo de payout -----------------------
    if let Some(details) = payout_method_data {
        req.payout_type = Some(match details {
            PayoutMethodData::BankTransfer(_) => PayoutType::Bank,
            PayoutMethodData::Card(_)         => PayoutType::Card,
            PayoutMethodData::Wallet(_)       => PayoutType::Wallet,
            PayoutMethodData::CashPickup(_)   => {
                // Cash aún no existe en PayoutType => dejamos sin setear
                return req;
            }
        });

        // Por ahora NO llenamos `payout_method_data` para evitar
        // desajustes de tipos con structs de `crate::payouts`.
    }

    req
}
