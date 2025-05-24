//! Domain types for remittances

// Re-export remittance-specific data types from api_models
pub use api_models::remittances::{
    BankAccountType, BankTransferData, BeneficiaryDetails, CashPickupData, CardPayoutData,
    ComplianceCheckStatus, ComplianceStatus, CustomerDetails, DocumentStatus, ExchangeRateInfo,
    PayoutMethodData, RemittanceCreateRequest, RemittanceListRequest, RemittanceListResponse,
    RemittanceManualUpdateRequest, RemittancePayRequest, RemittancePurpose, RemittanceQuoteRequest,
    RemittanceQuoteResponse, RemittanceRetrieveRequest, RemittanceResponse, RemittanceStatus,
    RemittanceSyncRequest, RemittanceSyncResponse, RemittanceSyncResult, RemittanceUpdateRequest,
    RequiredDocument, SenderDetails, WalletPayoutData, WalletType,
};

// Re-export payment-related types for remittance funding (sender payment)
pub use api_models::payments::{
    Address, Amount, PaymentMethodData, PaymentMethodDataRequest, PhoneDetails,
};

// Re-export payment method data types that can be used for remittance funding
pub use hyperswitch_domain_models::payment_method_data::{
    BankDebitData, BankRedirectData, BankTransferData as PaymentBankTransferData, Card,
    CardRedirectData, CardToken, PaymentMethodData as DomainPaymentMethodData, UpiCollectData,
    UpiData, UpiIntentData, VoucherData, WalletData,
};

// Re-export payout-related types for remittance delivery (beneficiary payout)
#[cfg(feature = "payouts")]
pub use api_models::payouts::{
    AchBankTransfer, BacsBankTransfer, Bank, CardPayout, PayoutCreateRequest,
    PayoutMethodData as PayoutMethod, PixBankTransfer, SepaBankTransfer, Wallet as PayoutWallet,
};

// Re-export core domain types
pub use hyperswitch_domain_models::merchant_account::MerchantAccount;
pub use hyperswitch_domain_models::business_profile::Profile;

// Re-export router flow types
pub use hyperswitch_domain_models::router_flow_types::{Execute, RSync};
pub use hyperswitch_domain_models::router_data::{ConnectorAuthType, ErrorResponse, RouterData};

// Re-export common ID types
pub use common_utils::id_type::{CustomerId, MerchantId, PaymentId, ProfileId};

// Define type aliases for remittance-specific IDs (until properly implemented)
pub type RemittanceId = String;
pub type PayoutId = String;

// Re-export common utility types
pub use common_utils::types::{MinorUnit, TimeRange};
pub use common_utils::pii::SecretSerdeValue;
pub use common_utils::errors::CustomResult;
pub use common_utils::date_time;

// Re-export time and date types
pub use time::{Date, OffsetDateTime, PrimitiveDateTime};

// Re-export decimal type for exchange rates
pub use rust_decimal::Decimal;

// Re-export masking types for sensitive data
pub use masking::{Maskable, Secret};

// Re-export error types
pub use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;

// Re-export connector and configuration types
pub use hyperswitch_domain_models::configs::Connectors;
pub use hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount;

// Re-export enum types
pub use api_models::enums::{CountryAlpha2, Currency, FutureUsage};

// Re-export database models (conditional on remittances feature)
#[cfg(feature = "remittances")]
pub use diesel_models::remittance::{
    Remittance, RemittanceNew, RemittancePayment, RemittancePaymentNew, RemittancePayout,
    RemittancePayoutNew, RemittanceUpdate, RemittanceUpdateInternal,
};

// Type aliases for remittance operations
pub type RemittanceAmount = MinorUnit;
pub type ExchangeRate = Decimal;

// Temporary router data type aliases (until implemented in hyperswitch_domain_models)
// TODO: Implement these types in hyperswitch_domain_models::router_request_types and router_response_types
pub type RemittancesData = RemittanceCreateRequest;
pub type RemittancesResponseData = RemittanceResponse;

// Re-export logging utilities
pub use router_env::logger;

// Re-export payment integration types (for funding remittances)
pub use hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt;
pub use hyperswitch_domain_models::payments::PaymentIntent;

// Re-export authentication types
pub use hyperswitch_domain_models::router_data::AccessToken;
pub use hyperswitch_domain_models::api_keys::ApiKey;

// Re-export organization types
pub use hyperswitch_domain_models::organization::Organization;

// Re-export event and metric types
pub use hyperswitch_domain_models::events::ApiEventMetric;
pub use common_enums::enums::{EventClass, EventType};

// Conditional feature-based exports
#[cfg(feature = "frm")]
pub use hyperswitch_domain_models::fraud_check::FraudCheck;

#[cfg(feature = "kms")]
pub use external_services::kms;

#[cfg(feature = "redis")]
pub use common_utils::cache;

// Additional utility types
pub use serde_json::Value as JsonValue;
pub use uuid::Uuid;