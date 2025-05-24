//! Analítica de remesas: filtros, dimensiones y métricas.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use common_utils::{
    id_type,
    types::TimeRange,                // único tipo externo que realmente usamos
};

use crate::enums::{
    AttemptStatus, AuthenticationType, CardNetwork, Connector, Currency,
    PaymentMethodType,
};

#[cfg(feature = "remittances")]
use crate::enums::RemittanceMethod;

use super::{ForexMetric, NameDescription};

/// Filtros disponibles para la API de métricas de remesas.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct RemittanceFilters {
    #[serde(default)] pub currency: Vec<Currency>,
    #[serde(default)] pub status: Vec<AttemptStatus>,
    #[serde(default)] pub connector: Vec<Connector>,
    #[serde(default)] pub auth_type: Vec<AuthenticationType>,
    #[cfg(feature = "remittances")]
    #[serde(default)] pub remittance_method: Vec<RemittanceMethod>,
    #[serde(default)] pub payment_method_type: Vec<PaymentMethodType>,
    #[serde(default)] pub client_source: Vec<String>,
    #[serde(default)] pub client_version: Vec<String>,
    #[serde(default)] pub card_network: Vec<CardNetwork>,
    #[serde(default)] pub profile_id: Vec<id_type::ProfileId>,
    #[serde(default)] pub merchant_id: Vec<id_type::MerchantId>,
    #[serde(default)] pub card_last_4: Vec<String>,
    #[serde(default)] pub card_issuer: Vec<String>,
    #[serde(default)] pub error_reason: Vec<String>,
    #[serde(default)] pub first_attempt: Vec<bool>,
}

/// Ejes (dimensiones) sobre los que se pueden agrupar las métricas.
#[derive(
    Debug, serde::Serialize, serde::Deserialize, strum::AsRefStr, PartialEq,
    PartialOrd, Eq, Ord, strum::Display, strum::EnumIter, Clone, Copy,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RemittanceDimensions {
    Connector,
    RemittanceMethod,
    PaymentMethodType,
    Currency,
    #[strum(serialize = "authentication_type")]
    #[serde(rename = "authentication_type")]
    AuthType,
    #[strum(serialize = "status")]
    #[serde(rename = "status")]
    RemittanceStatus,
    ClientSource,
    ClientVersion,
    ProfileId,
    CardNetwork,
    MerchantId,
    #[strum(serialize = "card_last_4")]
    #[serde(rename = "card_last_4")]
    CardLast4,
    CardIssuer,
    ErrorReason,
}

/// Métricas soportadas para remesas.
#[derive(
    Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize,
    strum::Display, strum::EnumIter, strum::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RemittanceMetrics {
    RemittanceSuccessRate,
    RemittanceCount,
    RemittanceSuccessCount,
    RemittanceProcessedAmount,
    AvgTicketSize,
    RetriesCount,
    ConnectorSuccessRate,
    SessionizedRemittanceSuccessRate,
    SessionizedRemittanceCount,
    SessionizedRemittanceSuccessCount,
    SessionizedRemittanceProcessedAmount,
    SessionizedAvgTicketSize,
    SessionizedRetriesCount,
    SessionizedConnectorSuccessRate,
    RemittancesDistribution,
    FailureReasons,
}

impl ForexMetric for RemittanceMetrics {
    fn is_forex_metric(&self) -> bool {
        matches!(
            self,
            Self::RemittanceProcessedAmount
                | Self::AvgTicketSize
                | Self::SessionizedRemittanceProcessedAmount
                | Self::SessionizedAvgTicketSize
        )
    }
}

/// Estructura auxiliar para mostrar fallos y porcentajes.
#[derive(Debug, Default, serde::Serialize)]
pub struct ErrorResult {
    pub reason: String,
    pub count: i64,
    pub percentage: f64,
}

/// Distribuciones específicas.
#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, strum::Display,
    strum::EnumIter, strum::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RemittanceDistributions {
    #[strum(serialize = "error_message")]
    RemittanceErrorMessage,
}

/// Alias internos para comportamientos concretos de las métricas.
pub mod metric_behaviour {
    pub struct RemittanceSuccessRate;
    pub struct RemittanceCount;
    pub struct RemittanceSuccessCount;
    pub struct RemittanceProcessedAmount;
    pub struct AvgTicketSize;
}

impl From<RemittanceMetrics> for NameDescription {
    fn from(value: RemittanceMetrics) -> Self {
        Self { name: value.to_string(), desc: String::new() }
    }
}
impl From<RemittanceDimensions> for NameDescription {
    fn from(value: RemittanceDimensions) -> Self {
        Self { name: value.to_string(), desc: String::new() }
    }
}

/// Identificador único de un bucket de métricas.
#[derive(Debug, serde::Serialize, Eq)]
pub struct RemittanceMetricsBucketIdentifier {
    pub currency: Option<Currency>,
    pub status: Option<AttemptStatus>,
    pub connector: Option<String>,
    #[serde(rename = "authentication_type")]
    pub auth_type: Option<AuthenticationType>,
    pub remittance_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub profile_id: Option<String>,
    pub card_network: Option<String>,
    pub merchant_id: Option<String>,
    pub card_last_4: Option<String>,
    pub card_issuer: Option<String>,
    pub error_reason: Option<String>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    // Para FE
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl Hash for RemittanceMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.currency.hash(state);
        self.status.map(|i| i.to_string()).hash(state);
        self.connector.hash(state);
        self.auth_type.map(|i| i.to_string()).hash(state);
        self.remittance_method.hash(state);
        self.payment_method_type.hash(state);
        self.client_source.hash(state);
        self.client_version.hash(state);
        self.profile_id.hash(state);
        self.card_network.hash(state);
        self.merchant_id.hash(state);
        self.card_last_4.hash(state);
        self.card_issuer.hash(state);
        self.error_reason.hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for RemittanceMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

/// Valores asociados a un bucket de métricas.
#[derive(Debug, serde::Serialize)]
pub struct RemittanceMetricsBucketValue {
    pub remittance_success_rate: Option<f64>,
    pub remittance_count: Option<u64>,
    pub remittance_success_count: Option<u64>,
    pub remittance_processed_amount: Option<u64>,
    pub remittance_processed_amount_in_usd: Option<u64>,
    pub remittance_processed_count: Option<u64>,
    pub remittance_processed_amount_without_smart_retries: Option<u64>,
    pub remittance_processed_amount_without_smart_retries_usd: Option<u64>,
    pub remittance_processed_count_without_smart_retries: Option<u64>,
    pub avg_ticket_size: Option<f64>,
    pub remittance_error_message: Option<Vec<ErrorResult>>,
    pub retries_count: Option<u64>,
    pub retries_amount_processed: Option<u64>,
    pub connector_success_rate: Option<f64>,
    pub remittances_success_rate_distribution: Option<f64>,
    pub remittances_success_rate_distribution_without_smart_retries: Option<f64>,
    pub remittances_success_rate_distribution_with_only_retries: Option<f64>,
    pub remittances_failure_rate_distribution: Option<f64>,
    pub remittances_failure_rate_distribution_without_smart_retries: Option<f64>,
    pub remittances_failure_rate_distribution_with_only_retries: Option<f64>,
    pub failure_reason_count: Option<u64>,
    pub failure_reason_count_without_smart_retries: Option<u64>,
}

/// Wrapper de respuesta devuelto por la API de analytics.
#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)] pub values: RemittanceMetricsBucketValue,
    #[serde(flatten)] pub dimensions: RemittanceMetricsBucketIdentifier,
}
