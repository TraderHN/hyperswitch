// crates/hyperswitch_domain_models/src/remittances.rs

use common_utils::{
    errors::CustomResult,
    id_type::{MerchantId, PaymentId, ProfileId},
    pii::SecretSerdeValue,
    types::MinorUnit,
    types::keymanager::KeyManagerState,
};
use common_enums::enums::{self as enums, Currency, RemittanceStatus, MerchantStorageScheme};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use async_trait::async_trait;
use crate::merchant_key_store::MerchantKeyStore;
use diesel_models::remittances::{
    RemittanceUpdateInternal, RemittancePaymentUpdate, RemittancePayoutUpdate,
};

/// Alias local hasta que `PayoutId` se exponga en `common_utils::id_type`
pub type PayoutId = String;

/// Modelo de dominio para remesas
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Remittance {
    pub id: i32,
    pub remittance_id: Uuid,
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

/// Información de tasa de cambio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateInfo {
    pub rate: f64,
    pub markup: Option<f64>,
    pub source_currency: Currency,
    pub destination_currency: Currency,
    pub valid_until: Option<OffsetDateTime>,
}

/// Detalles del remitente
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SenderDetails {
    pub name: String,
    pub customer_id: Option<String>,
    pub customer_details: Option<CustomerDetails>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub phone_country_code: Option<String>,
    pub address: Option<Address>,
    pub payment_method_data: Option<PaymentMethodData>,
}

/// Detalles del beneficiario
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeneficiaryDetails {
    pub name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub customer_id: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub phone_country_code: Option<String>,
    pub address: Option<Address>,
    pub payout_details: Option<PayoutMethodData>,
    pub relationship: Option<String>,
}

/// Datos comunes de cliente
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomerDetails {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub phone_country_code: Option<String>,
}

/// Dirección genérica
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

/// Métodos de pago
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardData {
    pub card_number: String,
    pub card_exp_month: String,
    pub card_exp_year: String,
    pub card_holder_name: Option<String>,
    pub card_cvc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankData {
    pub account_number: String,
    pub bank_name: Option<String>,
    pub bank_code: Option<String>,
    pub account_type: Option<String>,
    pub account_holder_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletData {
    pub wallet_id: String,
    pub wallet_type: WalletType,
    pub wallet_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoData {
    pub crypto_address: String,
    pub crypto_currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiData {
    pub upi_id: String,
    pub vpa_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayLaterData {
    pub provider: String,
    pub customer_id: Option<String>,
    pub redirect_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftCardData {
    pub gift_card_number: String,
    pub gift_card_pin: Option<String>,
    pub gift_card_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoucherData {
    pub voucher_number: String,
    pub voucher_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashData {
    pub payment_reference: String,
}

/// Métodos de payout
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PayoutMethodData {
    BankTransfer(BankTransferData),
    Card(CardPayoutData),
    Wallet(WalletPayoutData),
    CashPickup(CashPickupData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransferData {
    pub account_number: String,
    pub routing_number: Option<String>,
    pub bic: Option<String>,
    pub iban: Option<String>,
    pub bank_name: Option<String>,
    pub bank_country: Option<enums::CountryAlpha2>,
    pub bank_address: Option<String>,
    pub account_type: Option<BankAccountType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankAccountType {
    Checking,
    Savings,
    Corporate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPayoutData {
    pub card_token: String,
    pub last4: Option<String>,
    pub card_network: Option<enums::CardNetwork>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletPayoutData {
    pub wallet_type: WalletType,
    pub wallet_id: String,
    pub provider_details: Option<SecretSerdeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashPickupData {
    pub location_code: String,
    pub location_name: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletType {
    MobileMoney,
    DigitalWallet,
    BankWallet,
    CryptoWallet,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub status: ComplianceCheckStatus,
    pub required_info: Option<Vec<String>>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceCheckStatus {
    Pending,
    Reviewing,
    Approved,
    Rejected,
    AdditionalInfoRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredDocument {
    pub document_type: String,
    pub description: Option<String>,
    pub status: DocumentStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Required,
    Submitted,
    Approved,
    Rejected,
}

/// Modelo de dominio para pagos de remesa
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemittancePayment {
    pub remittance_id: Uuid,
    pub payment_id: PaymentId,
    pub connector: String,
    pub connector_transaction_id: Option<String>,
    pub status: RemittanceStatus,
    pub failure_reason: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Modelo de dominio para liquidaciones de remesa
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemittancePayout {
    pub remittance_id: Uuid,
    pub payout_id: PayoutId,
    pub connector: String,
    pub connector_transaction_id: Option<String>,
    pub status: RemittanceStatus,
    pub failure_reason: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// -----------------------------------------------------------------------------
// Traits de interfaz de almacenamiento
// -----------------------------------------------------------------------------

/// Almacén de Remittances
#[async_trait]
pub trait RemittanceInterface {
    /// Tipo de error devuelto por el store
    type Error;

    /// Busca una remesa por su UUID
    async fn find_remittance_by_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        remittance_id: &Uuid,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Remittance, Self::Error>;

    /// Busca una remesa por (merchant_id, reference)
    async fn find_remittance_by_merchant_id_reference(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &MerchantId,
        reference: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Remittance, Self::Error>;

    /// Inserta una nueva remesa
    async fn insert_remittance(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        remittance: Remittance,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Remittance, Self::Error>;

    /// Actualiza una remesa existente
    async fn update_remittance(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        remittance: Remittance,
        remittance_update: RemittanceUpdateInternal,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Remittance, Self::Error>;

    /// Filtra remesas por merchant_id + profile_id
    async fn find_remittances_by_merchant_id_profile_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &MerchantId,
        profile_id: &ProfileId,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<Remittance>, Self::Error>;

    /// Filtra remesas por merchant_id + estado
    async fn find_remittances_by_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &MerchantId,
        status: RemittanceStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<Remittance>, Self::Error>;
}

/// Almacén de pagos de remesa
#[async_trait]
pub trait RemittancePaymentInterface {
    type Error;

    /// Busca el pago de remesa asociado a una remittance_id
    async fn find_remittance_payment_by_remittance_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        remittance_id: &Uuid,
    ) -> CustomResult<RemittancePayment, Self::Error>;

    /// Inserta un nuevo pago de remesa
    async fn insert_remittance_payment(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment: RemittancePayment,
    ) -> CustomResult<RemittancePayment, Self::Error>;

    /// Actualiza un pago de remesa existente
    async fn update_remittance_payment(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment: RemittancePayment,
        update: RemittancePaymentUpdate,
    ) -> CustomResult<RemittancePayment, Self::Error>;
}

/// Almacén de liquidaciones de remesa (payouts)
#[async_trait]
pub trait RemittancePayoutInterface {
    type Error;

    /// Busca la liquidación (payout) asociada a una remittance_id
    async fn find_remittance_payout_by_remittance_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        remittance_id: &Uuid,
    ) -> CustomResult<RemittancePayout, Self::Error>;

    /// Inserta una nueva liquidación de remesa
    async fn insert_remittance_payout(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payout: RemittancePayout,
    ) -> CustomResult<RemittancePayout, Self::Error>;

    /// Actualiza una liquidación de remesa existente
    async fn update_remittance_payout(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payout: RemittancePayout,
        update: RemittancePayoutUpdate,
    ) -> CustomResult<RemittancePayout, Self::Error>;
}
