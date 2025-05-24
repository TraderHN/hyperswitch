use common_utils::{
    crypto::Encryptable,
    id_type::{MerchantId, PaymentId, PayoutId, ProfileId},
    pii::SecretSerdeValue,
    types::MinorUnit,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::enums::{self, Currency, RemittanceStatus};

/// Modelo de dominio para remesas
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Remittance {
    pub id: i32,
    pub remittance_id: String,
    pub merchant_id: MerchantId,
    pub profile_id: ProfileId,
    pub connector: String,
    pub merchant_connector_id: Option<String>,
    pub amount: MinorUnit,
    pub source_currency: Currency,
    pub destination_currency: Currency,
    pub source_amount: MinorUnit,
    pub destination_amount: MinorUnit,
    pub exchange_rate: Option<ExchangeRateInfo>,
    pub sender_details: SenderDetails,
    pub beneficiary_details: BeneficiaryDetails,
    pub remittance_date: String,
    pub reference: String,
    pub purpose: Option<RemittancePurpose>,
    pub status: RemittanceStatus,
    pub failure_reason: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<SecretSerdeValue>,
    pub client_secret: Option<String>,
    pub payment_id: Option<PaymentId>,
    pub payout_id: Option<PayoutId>,
    pub payment_connector_transaction_id: Option<String>,
    pub payout_connector_transaction_id: Option<String>,
    pub estimated_delivery_time: Option<OffsetDateTime>,
    pub actual_delivery_time: Option<OffsetDateTime>,
    pub compliance_status: Option<ComplianceStatus>,
    pub required_documents: Option<Vec<RequiredDocument>>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Propósito de la remesa
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Tipo de wallet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Información de tasa de cambio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateInfo {
    /// Exchange rate (1 source = X destination)
    pub rate: f64,
    /// Optional markup percentage
    pub markup: Option<f64>,
    /// Source currency
    pub source_currency: Currency,
    /// Destination currency
    pub destination_currency: Currency,
    /// Rate valid until timestamp
    pub valid_until: Option<OffsetDateTime>,
}

/// Detalles del remitente
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SenderDetails {
    /// Full name of sender
    pub name: String,
    /// Optional customer ID if sender is an existing customer
    pub customer_id: Option<String>,
    /// Optional customer details
    pub customer_details: Option<CustomerDetails>,
    /// Optional email
    pub email: Option<String>,
    /// Optional phone number
    pub phone: Option<String>,
    /// Optional phone country code
    pub phone_country_code: Option<String>,
    /// Optional address
    pub address: Option<Address>,
    /// Payment method data to use for funding
    pub payment_method_data: Option<PaymentMethodData>,
}

/// Detalles del beneficiario
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeneficiaryDetails {
    /// Full name of beneficiary
    pub name: String,
    /// Optional first name
    pub first_name: Option<String>,
    /// Optional last name
    pub last_name: Option<String>,
    /// Optional customer ID
    pub customer_id: Option<String>,
    /// Optional email
    pub email: Option<String>,
    /// Optional phone number
    pub phone: Option<String>,
    /// Optional phone country code
    pub phone_country_code: Option<String>,
    /// Optional address
    pub address: Option<Address>,
    /// Payout method details (bank account, wallet, etc.)
    pub payout_details: Option<PayoutMethodData>,
    /// Optional relationship to sender
    pub relationship: Option<String>,
}

/// Detalles del cliente
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomerDetails {
    /// Full name
    pub name: Option<String>,
    /// Email
    pub email: Option<String>,
    /// Phone number
    pub phone: Option<String>,
    /// Phone country code
    pub phone_country_code: Option<String>,
}

/// Dirección
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub line3: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Método de pago para datos de remesa
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PaymentMethodData {
    Card(CardData),
    Bank(BankData),
    Wallet(WalletData),
    Crypto(CryptoData),
    Upi(UpiData),
    PayLater(PayLaterData),
    GiftCard(GiftCardData),
    Voucher(VoucherData),
    Cash(CashData),
}

/// Datos de tarjeta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardData {
    pub card_number: String,
    pub card_exp_month: String,
    pub card_exp_year: String,
    pub card_holder_name: Option<String>,
    pub card_cvc: String,
}

/// Datos bancarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankData {
    pub account_number: String,
    pub bank_name: Option<String>,
    pub bank_code: Option<String>,
    pub account_type: Option<String>,
    pub account_holder_name: Option<String>,
}

/// Datos de wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletData {
    pub wallet_id: String,
    pub wallet_type: WalletType,
    pub wallet_reference: Option<String>,
}

/// Datos crypto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoData {
    pub crypto_address: String,
    pub crypto_currency: String,
}

/// Datos UPI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiData {
    pub upi_id: String,
    pub vpa_id: Option<String>,
}

/// Datos PayLater
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayLaterData {
    pub provider: String,
    pub customer_id: Option<String>,
    pub redirect_url: Option<String>,
}

/// Datos tarjeta regalo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftCardData {
    pub gift_card_number: String,
    pub gift_card_pin: Option<String>,
    pub gift_card_provider: String,
}

/// Datos voucher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoucherData {
    pub voucher_number: String,
    pub voucher_provider: String,
}

/// Datos efectivo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashData {
    pub payment_reference: String,
}

/// Método de payout para remesas
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransferData {
    /// Account number
    pub account_number: String,
    /// Routing/Sort/BSB code
    pub routing_number: Option<String>,
    /// BIC/SWIFT code
    pub bic: Option<String>,
    /// IBAN
    pub iban: Option<String>,
    /// Bank name
    pub bank_name: Option<String>,
    /// Bank country
    pub bank_country: Option<enums::CountryAlpha2>,
    /// Bank address
    pub bank_address: Option<String>,
    /// Account type
    pub account_type: Option<BankAccountType>,
}

/// Bank account type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPayoutData {
    /// Card token for push-to-card
    pub card_token: String,
    /// Last four digits
    pub last4: Option<String>,
    /// Card network
    pub card_network: Option<enums::CardNetwork>,
}

/// Digital wallet data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletPayoutData {
    /// Wallet type
    pub wallet_type: WalletType,
    /// Wallet ID/phone number
    pub wallet_id: String,
    /// Provider-specific data
    pub provider_details: Option<SecretSerdeValue>,
}

/// Cash pickup data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashPickupData {
    /// Pickup location code
    pub location_code: String,
    /// Pickup location name
    pub location_name: Option<String>,
    /// Pickup country
    pub country: Option<enums::CountryAlpha2>,
    /// Additional pickup instructions
    pub instructions: Option<String>,
}

/// Compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    /// Current compliance status
    pub status: ComplianceCheckStatus,
    /// Required information list
    pub required_info: Option<Vec<String>>,
    /// Status message
    pub message: Option<String>,
}

/// Compliance check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredDocument {
    /// Document type
    pub document_type: String,
    /// Description
    pub description: Option<String>,
    /// Document status
    pub status: DocumentStatus,
}

/// Document status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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