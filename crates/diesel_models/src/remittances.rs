//! Definiciones y funcionalidades para las remesas
//! Este módulo implementa las estructuras y lógica para el procesamiento de remesas

// Importaciones estándar
use serde::{Deserialize, Serialize};
use time::{Date, PrimitiveDateTime}; 
use uuid::Uuid;
use rust_decimal::Decimal;
use std::str::FromStr; // Importación necesaria para usar from_str

// Importaciones de Diesel
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

// Importaciones de tipos personalizados
use common_utils::id_type::{MerchantId, ProfileId};
use common_utils::pii::SecretSerdeValue;
use common_utils::date_time;

// Importación del esquema de la base de datos
use crate::schema_v2::{remittances, remittance_payments, remittance_payouts};

/// Estado de remesa
#[cfg(feature = "remittances")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RemittanceStatus {
    /// Remesa recién creada
    Created,
    /// Procesamiento de pago iniciado
    PaymentInitiated,
    /// Pago procesado correctamente
    PaymentProcessed,
    /// Procesamiento de liquidación iniciado
    PayoutInitiated,
    /// Remesa completada correctamente
    Completed,
    /// Remesa fallida
    Failed,
    /// Remesa cancelada
    Cancelled,
}

#[cfg(feature = "remittances")]
impl std::fmt::Display for RemittanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            Self::Created => "created",
            Self::PaymentInitiated => "payment_initiated",
            Self::PaymentProcessed => "payment_processed",
            Self::PayoutInitiated => "payout_initiated",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        };
        write!(f, "{}", status)
    }
}

#[cfg(feature = "remittances")]
impl RemittanceStatus {
    pub fn from_string(status: &str) -> Option<Self> {
        match status {
            "created" => Some(Self::Created),
            "payment_initiated" => Some(Self::PaymentInitiated),
            "payment_processed" => Some(Self::PaymentProcessed),
            "payout_initiated" => Some(Self::PayoutInitiated),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// Modelo que mapea la tabla `remittances`
#[cfg(feature = "remittances")]
#[derive(
    Clone,
    Debug,
    Eq,
    Identifiable,
    Queryable,
    Selectable,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = remittances, primary_key(id))]
pub struct Remittance {
    pub id: Uuid,
    pub merchant_id: MerchantId,
    pub profile_id: ProfileId,
    pub amount: i64,
    pub source_currency: String,
    pub destination_currency: String,
    pub source_amount: Option<i64>,
    pub destination_amount: Option<i64>,
    pub exchange_rate: Option<Decimal>,
    pub reference: String,
    pub purpose: Option<String>,
    pub status: String,
    pub failure_reason: Option<String>,
    pub sender_details: SecretSerdeValue,
    pub beneficiary_details: SecretSerdeValue,
    pub return_url: Option<String>,
    pub metadata: Option<SecretSerdeValue>,
    pub connector: String,
    pub client_secret: Option<String>,
    pub remittance_date: Date,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

/// Parámetros para insertar una nueva fila en `remittances`
#[cfg(feature = "remittances")]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Insertable,
    router_derive::DebugAsDisplay,
    Serialize,
    Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = remittances)]
pub struct RemittanceNew {
    pub id: Uuid,
    pub merchant_id: MerchantId,
    pub profile_id: ProfileId,
    pub amount: i64,
    pub source_currency: String,
    pub destination_currency: String,
    pub source_amount: Option<i64>,
    pub destination_amount: Option<i64>,
    pub exchange_rate: Option<Decimal>,
    pub reference: String,
    pub purpose: Option<String>,
    pub status: String,
    pub failure_reason: Option<String>,
    pub sender_details: SecretSerdeValue,
    pub beneficiary_details: SecretSerdeValue,
    pub return_url: Option<String>,
    pub metadata: Option<SecretSerdeValue>,
    pub connector: String,
    pub client_secret: Option<String>,
    pub remittance_date: Date,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

/// Actualizaciones de remesa
#[cfg(feature = "remittances")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemittanceUpdate {
    /// Actualización general de estado y posible razón de fallo
    Update {
        status: String,
        failure_reason: Option<String>,
        updated_at: PrimitiveDateTime,
    },
    /// Actualización de metadatos y propósito
    MetadataUpdate {
        metadata: Option<SecretSerdeValue>,
        purpose: Option<String>,
        updated_at: PrimitiveDateTime,
    },
    /// Actualización exclusiva del estado
    StatusUpdate {
        status: String,
        updated_at: PrimitiveDateTime,
    },
    /// Actualización en caso de error
    ErrorUpdate {
        status: Option<String>,
        failure_reason: Option<String>,
        updated_at: PrimitiveDateTime,
    },
    /// Actualización manual por operador
    ManualUpdate {
        status: Option<String>,
        failure_reason: Option<String>,
        updated_at: PrimitiveDateTime,
    },
}

/// Campos que pueden actualizarse en `remittances`
#[cfg(feature = "remittances")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = remittances)]
pub struct RemittanceUpdateInternal {
    pub merchant_id: Option<MerchantId>,
    pub profile_id: Option<ProfileId>,
    pub amount: Option<i64>,
    pub source_currency: Option<String>,
    pub destination_currency: Option<String>,
    pub source_amount: Option<i64>,
    pub destination_amount: Option<i64>,
    pub exchange_rate: Option<Decimal>,
    pub reference: Option<String>,
    pub purpose: Option<String>,
    pub status: Option<String>,
    pub failure_reason: Option<String>,
    pub sender_details: Option<SecretSerdeValue>,
    pub beneficiary_details: Option<SecretSerdeValue>,
    pub return_url: Option<String>,
    pub metadata: Option<SecretSerdeValue>,
    pub connector: Option<String>,
    pub client_secret: Option<String>,
    pub remittance_date: Option<Date>,
    pub updated_at: Option<PrimitiveDateTime>,
}

#[cfg(feature = "remittances")]
impl RemittanceUpdateInternal {
    pub fn create_remittance(self, source: Remittance) -> Remittance {
        Remittance {
            merchant_id: self.merchant_id.unwrap_or(source.merchant_id),
            profile_id: self.profile_id.unwrap_or(source.profile_id),
            amount: self.amount.unwrap_or(source.amount),
            source_currency: self.source_currency.unwrap_or(source.source_currency),
            destination_currency: self.destination_currency.unwrap_or(source.destination_currency),
            source_amount: self.source_amount.or(source.source_amount),
            destination_amount: self.destination_amount.or(source.destination_amount),
            exchange_rate: self.exchange_rate.or(source.exchange_rate),
            reference: self.reference.unwrap_or(source.reference),
            purpose: self.purpose.or(source.purpose),
            status: self.status.unwrap_or(source.status),
            failure_reason: self.failure_reason.or(source.failure_reason),
            sender_details: self.sender_details.unwrap_or(source.sender_details),
            beneficiary_details: self.beneficiary_details.unwrap_or(source.beneficiary_details),
            return_url: self.return_url.or(source.return_url),
            metadata: self.metadata.or(source.metadata),
            connector: self.connector.unwrap_or(source.connector),
            client_secret: self.client_secret.or(source.client_secret),
            remittance_date: self.remittance_date.unwrap_or(source.remittance_date),
            updated_at: self.updated_at.or(source.updated_at),
            ..source
        }
    }
}

#[cfg(feature = "remittances")]
impl From<RemittanceUpdate> for RemittanceUpdateInternal {
    fn from(remittance_update: RemittanceUpdate) -> Self {
        match remittance_update {
            RemittanceUpdate::Update {
                status,
                failure_reason,
                updated_at,
            } => Self {
                status: Some(status),
                failure_reason,
                updated_at: Some(updated_at),
                merchant_id: None,
                profile_id: None,
                amount: None,
                source_currency: None,
                destination_currency: None,
                source_amount: None,
                destination_amount: None,
                exchange_rate: None,
                reference: None,
                purpose: None,
                sender_details: None,
                beneficiary_details: None,
                return_url: None,
                metadata: None,
                connector: None,
                client_secret: None,
                remittance_date: None,
            },
            RemittanceUpdate::MetadataUpdate {
                metadata,
                purpose,
                updated_at,
            } => Self {
                metadata,
                purpose,
                updated_at: Some(updated_at),
                status: None,
                failure_reason: None,
                merchant_id: None,
                profile_id: None,
                amount: None,
                source_currency: None,
                destination_currency: None,
                source_amount: None,
                destination_amount: None,
                exchange_rate: None,
                reference: None,
                sender_details: None,
                beneficiary_details: None,
                return_url: None,
                connector: None,
                client_secret: None,
                remittance_date: None,
            },
            RemittanceUpdate::StatusUpdate {
                status,
                updated_at,
            } => Self {
                status: Some(status),
                updated_at: Some(updated_at),
                merchant_id: None,
                profile_id: None,
                amount: None,
                source_currency: None,
                destination_currency: None,
                source_amount: None,
                destination_amount: None,
                exchange_rate: None,
                reference: None,
                purpose: None,
                failure_reason: None,
                sender_details: None,
                beneficiary_details: None,
                return_url: None,
                metadata: None,
                connector: None,
                client_secret: None,
                remittance_date: None,
            },
            RemittanceUpdate::ErrorUpdate {
                status,
                failure_reason,
                updated_at,
            } => Self {
                status,
                failure_reason,
                updated_at: Some(updated_at),
                merchant_id: None,
                profile_id: None,
                amount: None,
                source_currency: None,
                destination_currency: None,
                source_amount: None,
                destination_amount: None,
                exchange_rate: None,
                reference: None,
                purpose: None,
                sender_details: None,
                beneficiary_details: None,
                return_url: None,
                metadata: None,
                connector: None,
                client_secret: None,
                remittance_date: None,
            },
            RemittanceUpdate::ManualUpdate {
                status,
                failure_reason,
                updated_at,
            } => Self {
                status,
                failure_reason,
                updated_at: Some(updated_at),
                merchant_id: None,
                profile_id: None,
                amount: None,
                source_currency: None,
                destination_currency: None,
                source_amount: None,
                destination_amount: None,
                exchange_rate: None,
                reference: None,
                purpose: None,
                sender_details: None,
                beneficiary_details: None,
                return_url: None,
                metadata: None,
                connector: None,
                client_secret: None,
                remittance_date: None,
            },
        }
    }
}

#[cfg(feature = "remittances")]
impl RemittanceUpdate {
    pub fn apply_changeset(self, source: Remittance) -> Remittance {
        let RemittanceUpdateInternal {
            status,
            failure_reason,
            updated_at,
            metadata,
            purpose,
            ..
        } = self.into();
        
        Remittance {
            status: status.unwrap_or(source.status),
            failure_reason: failure_reason.or(source.failure_reason),
            updated_at,
            metadata: metadata.or(source.metadata),
            purpose: purpose.or(source.purpose),
            ..source
        }
    }

    pub fn build_error_update(
        status: Option<String>,
        failure_reason: Option<String>,
    ) -> Self {
        Self::ErrorUpdate {
            status,
            failure_reason,
            updated_at: date_time::now(),
        }
    }

    pub fn build_remittance_update(
        status: String,
    ) -> Self {
        Self::Update {
            status,
            failure_reason: None,
            updated_at: date_time::now(),
        }
    }

    pub fn build_status_update(
        status: impl Into<String>,
    ) -> Self {
        Self::StatusUpdate {
            status: status.into(),
            updated_at: date_time::now(),
        }
    }

    pub fn build_failure_update(
        failure_reason: impl Into<String>,
    ) -> Self {
        Self::Update {
            status: RemittanceStatus::Failed.to_string(),
            failure_reason: Some(failure_reason.into()),
            updated_at: date_time::now(),
        }
    }
}

/// Pago que fondea la remesa (1-a-1 con `remittances`)
#[cfg(feature = "remittances")]
#[derive(
    Clone,
    Debug,
    Eq,
    Identifiable,
    Queryable,
    Selectable,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = remittance_payments, primary_key(remittance_id))]
pub struct RemittancePayment {
    pub remittance_id: Uuid,
    pub payment_id: Option<String>,
    pub connector_txn_id: Option<String>,
    pub status: Option<String>,
    pub auth_type: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

/// Parámetros para insertar un nuevo pago
#[cfg(feature = "remittances")]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Insertable,
    router_derive::DebugAsDisplay,
    Serialize,
    Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = remittance_payments)]
pub struct RemittancePaymentNew {
    pub remittance_id: Uuid,
    pub payment_id: Option<String>,
    pub connector_txn_id: Option<String>,
    pub status: Option<String>,
    pub auth_type: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

/// Campos que pueden actualizarse en `remittance_payments`
#[cfg(feature = "remittances")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = remittance_payments)]
pub struct RemittancePaymentUpdate {
    pub payment_id: Option<String>,
    pub connector_txn_id: Option<String>,
    pub status: Option<String>,
    pub auth_type: Option<String>,
    pub updated_at: Option<PrimitiveDateTime>,
}

/// Payout que liquida la remesa (1-a-1 con `remittances`)
#[cfg(feature = "remittances")]
#[derive(
    Clone,
    Debug,
    Eq,
    Identifiable,
    Queryable,
    Selectable,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = remittance_payouts, primary_key(remittance_id))]
pub struct RemittancePayout {
    pub remittance_id: Uuid,
    pub payout_id: Option<String>,
    pub connector_txn_id: Option<String>,
    pub status: Option<String>,
    pub remittance_method: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

/// Parámetros para insertar un nuevo payout
#[cfg(feature = "remittances")]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Insertable,
    router_derive::DebugAsDisplay,
    Serialize,
    Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = remittance_payouts)]
pub struct RemittancePayoutNew {
    pub remittance_id: Uuid,
    pub payout_id: Option<String>,
    pub connector_txn_id: Option<String>,
    pub status: Option<String>,
    pub remittance_method: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

/// Campos que pueden actualizarse en `remittance_payouts`
#[cfg(feature = "remittances")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = remittance_payouts)]
pub struct RemittancePayoutUpdate {
    pub payout_id: Option<String>,
    pub connector_txn_id: Option<String>,
    pub status: Option<String>,
    pub remittance_method: Option<String>,
    pub updated_at: Option<PrimitiveDateTime>,
}

/// Estructura para flujos de trabajo con remesas
#[cfg(feature = "remittances")]
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RemittanceCoreWorkflow {
    pub remittance_id: Uuid,
    pub merchant_id: MerchantId,
}

#[cfg(feature = "remittances")]
impl Remittance {
    /// Obtiene el estado de la remesa como enum
    pub fn status_enum(&self) -> Option<RemittanceStatus> {
        RemittanceStatus::from_string(&self.status)
    }

    /// Obtiene el valor del tipo de cambio
    pub fn exchange_rate_value(&self) -> Option<&Decimal> {
        self.exchange_rate.as_ref()
    }
}

#[cfg(feature = "remittances")]
impl common_utils::events::ApiEventMetric for Remittance {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Remittance {
            remittance_id: self.id.to_string(),
            // merchant_id: Some(self.merchant_id.to_owned()) // Descomentar si el ApiEventsType lo soporta
        })
    }
}

// Función auxiliar para crear un ProfileId por defecto
#[cfg(feature = "remittances")]
fn get_default_profile_id() -> ProfileId {
    // Esta solución usa std::str::FromStr que vimos implementado para ProfileId
    // Y maneja el Result adecuadamente para producción
    match ProfileId::from_str("default_profile") {
        Ok(profile_id) => profile_id,
        Err(_) => {
            // En caso de error, intenta con una cadena vacía
            match ProfileId::from_str("") {
                Ok(profile_id) => profile_id,
                Err(_) => {
                    // Como último recurso, fallback a una string técnica
                    // que debería ser válida según las reglas de validación de ProfileId
                    ProfileId::from_str("pro_default_0000000000000000")
                        .expect("Failed to create a basic default ProfileId")
                }
            }
        }
    }
}

// API para facilitar la creación de remesas
#[cfg(feature = "remittances")]
impl RemittanceNew {
    /// Crea una nueva instancia de remesa con un ID generado aleatoriamente
    pub fn new() -> Self {
        Self::with_id(Uuid::new_v4())
    }

    /// Crea una nueva instancia con un ID específico
    pub fn with_id(id: Uuid) -> Self {
        Self {
            id,
            merchant_id: MerchantId::default(),
            profile_id: get_default_profile_id(), // Usar la función auxiliar
            amount: 0,
            source_currency: String::new(),
            destination_currency: String::new(),
            source_amount: None,
            destination_amount: None,
            exchange_rate: None,
            reference: String::new(),
            purpose: None,
            status: RemittanceStatus::Created.to_string(),
            failure_reason: None,
            sender_details: serde_json::json!({}).into(),
            beneficiary_details: serde_json::json!({}).into(),
            return_url: None,
            metadata: None,
            connector: String::new(),
            client_secret: None,
            remittance_date: date_time::now().date(),
            created_at: Some(date_time::now()),
            updated_at: Some(date_time::now()),
        }
    }

    /// Establece el tipo de cambio
    pub fn with_exchange_rate(mut self, rate: Decimal) -> Self {
        self.exchange_rate = Some(rate);
        self
    }

    /// Establece la moneda origen
    pub fn with_source_currency(mut self, currency: impl Into<String>) -> Self {
        self.source_currency = currency.into();
        self
    }

    /// Establece la moneda destino
    pub fn with_destination_currency(mut self, currency: impl Into<String>) -> Self {
        self.destination_currency = currency.into();
        self
    }

    /// Establece el monto de la remesa
    pub fn with_amount(mut self, amount: i64) -> Self {
        self.amount = amount;
        self
    }

    /// Establece el conector a utilizar
    pub fn with_connector(mut self, connector: impl Into<String>) -> Self {
        self.connector = connector.into();
        self
    }

    /// Establece la referencia
    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = reference.into();
        self
    }

    /// Establece el merchant_id
    pub fn with_merchant_id(mut self, merchant_id: MerchantId) -> Self {
        self.merchant_id = merchant_id;
        self
    }

    /// Establece el profile_id
    pub fn with_profile_id(mut self, profile_id: ProfileId) -> Self {
        self.profile_id = profile_id;
        self
    }

    /// Establece detalles del remitente
    pub fn with_sender_details(mut self, details: impl Serialize) -> Self {
        self.sender_details = match serde_json::to_value(details) {
            Ok(value) => value.into(),
            Err(_) => self.sender_details,
        };
        self
    }

    /// Establece detalles del beneficiario
    pub fn with_beneficiary_details(mut self, details: impl Serialize) -> Self {
        self.beneficiary_details = match serde_json::to_value(details) {
            Ok(value) => value.into(),
            Err(_) => self.beneficiary_details,
        };
        self
    }
}

// API para facilitar la creación de pagos para remesas
#[cfg(feature = "remittances")]
impl RemittancePaymentNew {
    /// Crea un nuevo pago para una remesa
    pub fn new(remittance_id: Uuid) -> Self {
        Self {
            remittance_id,
            payment_id: None,
            connector_txn_id: None,
            status: None,
            auth_type: None,
            created_at: Some(date_time::now()),
            updated_at: Some(date_time::now()),
        }
    }

    /// Establece el ID de pago
    pub fn with_payment_id(mut self, payment_id: impl Into<String>) -> Self {
        self.payment_id = Some(payment_id.into());
        self
    }

    /// Establece el ID de transacción del conector
    pub fn with_connector_txn_id(mut self, txn_id: impl Into<String>) -> Self {
        self.connector_txn_id = Some(txn_id.into());
        self
    }

    /// Establece el estado del pago
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Establece el tipo de autenticación
    pub fn with_auth_type(mut self, auth_type: impl Into<String>) -> Self {
        self.auth_type = Some(auth_type.into());
        self
    }
}

// API para facilitar la creación de payouts para remesas
#[cfg(feature = "remittances")]
impl RemittancePayoutNew {
    /// Crea un nuevo payout para una remesa
    pub fn new(remittance_id: Uuid) -> Self {
        Self {
            remittance_id,
            payout_id: None,
            connector_txn_id: None,
            status: None,
            remittance_method: None,
            created_at: Some(date_time::now()),
            updated_at: Some(date_time::now()),
        }
    }

    /// Establece el ID de payout
    pub fn with_payout_id(mut self, payout_id: impl Into<String>) -> Self {
        self.payout_id = Some(payout_id.into());
        self
    }

    /// Establece el ID de transacción del conector
    pub fn with_connector_txn_id(mut self, txn_id: impl Into<String>) -> Self {
        self.connector_txn_id = Some(txn_id.into());
        self
    }

    /// Establece el estado del payout
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Establece el método de remesa
    pub fn with_remittance_method(mut self, method: impl Into<String>) -> Self {
        self.remittance_method = Some(method.into());
        self
    }
}

// Tests unitarios
#[cfg(feature = "remittances")]
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_remittance_status_display() {
        assert_eq!(RemittanceStatus::Created.to_string(), "created");
        assert_eq!(RemittanceStatus::PaymentInitiated.to_string(), "payment_initiated");
        assert_eq!(RemittanceStatus::PaymentProcessed.to_string(), "payment_processed");
        assert_eq!(RemittanceStatus::PayoutInitiated.to_string(), "payout_initiated");
        assert_eq!(RemittanceStatus::Completed.to_string(), "completed");
        assert_eq!(RemittanceStatus::Failed.to_string(), "failed");
        assert_eq!(RemittanceStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_remittance_status_from_string() {
        assert_eq!(RemittanceStatus::from_string("created"), Some(RemittanceStatus::Created));
        assert_eq!(RemittanceStatus::from_string("payment_initiated"), Some(RemittanceStatus::PaymentInitiated));
        assert_eq!(RemittanceStatus::from_string("payment_processed"), Some(RemittanceStatus::PaymentProcessed));
        assert_eq!(RemittanceStatus::from_string("payout_initiated"), Some(RemittanceStatus::PayoutInitiated));
        assert_eq!(RemittanceStatus::from_string("completed"), Some(RemittanceStatus::Completed));
        assert_eq!(RemittanceStatus::from_string("failed"), Some(RemittanceStatus::Failed));
        assert_eq!(RemittanceStatus::from_string("cancelled"), Some(RemittanceStatus::Cancelled));
        assert_eq!(RemittanceStatus::from_string("invalid_status"), None);
    }
    
    #[test]
    fn test_backwards_compatibility() {
        let serialized_remittance = r#"{
            "id": "5f6b7d8c-4321-5678-9012-3456789abcde",
            "merchant_id": "merchant_123",
            "profile_id": "profile_456",
            "amount": 10000,
            "source_currency": "USD",
            "destination_currency": "HNL",
            "source_amount": 10000,
            "destination_amount": 245000,
            "exchange_rate": "24.5",
            "reference": "REM123456",
            "purpose": "Family support",
            "status": "created",
            "sender_details": {},
            "beneficiary_details": {},
            "connector": "Lafise",
            "remittance_date": "2025-05-15",
            "created_at": "2025-05-15T10:30:00Z",
            "updated_at": "2025-05-15T10:30:00Z"
        }"#;
        let deserialized = serde_json::from_str::<super::Remittance>(serialized_remittance);

        assert!(deserialized.is_ok());
        if let Ok(remittance) = deserialized {
            assert_eq!(remittance.status_enum(), Some(RemittanceStatus::Created));
        }
    }

    #[test]
    fn test_remittance_new() {
        let new_remittance = RemittanceNew::new()
            .with_merchant_id("merchant_123".into())
            .with_profile_id("profile_456".into())
            .with_amount(10000)
            .with_source_currency("USD")
            .with_destination_currency("HNL")
            .with_exchange_rate(Decimal::from_str("24.5").unwrap())
            .with_reference("REM123456")
            .with_connector("Lafise");

        assert_eq!(new_remittance.merchant_id, "merchant_123".into());
        assert_eq!(new_remittance.profile_id, "profile_456".into());
        assert_eq!(new_remittance.amount, 10000);
        assert_eq!(new_remittance.source_currency, "USD");
        assert_eq!(new_remittance.destination_currency, "HNL");
        assert_eq!(new_remittance.exchange_rate, Some(Decimal::from_str("24.5").unwrap()));
        assert_eq!(new_remittance.reference, "REM123456");
        assert_eq!(new_remittance.connector, "Lafise");
        assert_eq!(new_remittance.status, "created");
    }
    
    #[test]
    fn test_with_specific_id() {
        let uuid = Uuid::from_str("5f6b7d8c-4321-5678-9012-3456789abcde").unwrap();
        
        // Probar RemittanceNew::with_id
        let new_remittance = RemittanceNew::with_id(uuid);
        assert_eq!(new_remittance.id, uuid);
        
        // Probar RemittancePaymentNew::new
        let payment = RemittancePaymentNew::new(uuid);
        assert_eq!(payment.remittance_id, uuid);
        
        // Probar RemittancePayoutNew::new
        let payout = RemittancePayoutNew::new(uuid);
        assert_eq!(payout.remittance_id, uuid);
    }
    
    #[test]
    fn test_builder_methods() {
        let uuid = Uuid::from_str("5f6b7d8c-4321-5678-9012-3456789abcde").unwrap();
        
        // Probar payment builder
        let payment = RemittancePaymentNew::new(uuid)
            .with_payment_id("payment_123")
            .with_connector_txn_id("txn_456")
            .with_status("successful")
            .with_auth_type("3ds");
            
        assert_eq!(payment.payment_id, Some("payment_123".to_string()));
        assert_eq!(payment.connector_txn_id, Some("txn_456".to_string()));
        assert_eq!(payment.status, Some("successful".to_string()));
        assert_eq!(payment.auth_type, Some("3ds".to_string()));
        
        // Probar payout builder
        let payout = RemittancePayoutNew::new(uuid)
            .with_payout_id("payout_123")
            .with_connector_txn_id("txn_789")
            .with_status("successful")
            .with_remittance_method("bank_transfer");
            
        assert_eq!(payout.payout_id, Some("payout_123".to_string()));
        assert_eq!(payout.connector_txn_id, Some("txn_789".to_string()));
        assert_eq!(payout.status, Some("successful".to_string()));
        assert_eq!(payout.remittance_method, Some("bank_transfer".to_string()));
    }
    
    #[test]
    fn test_exchange_rate_value() {
        let exchange_rate = Decimal::from_str("24.5").unwrap();
        let remittance = Remittance {
            id: Uuid::new_v4(),
            merchant_id: "merchant_123".to_string().into(),
            profile_id: "profile_456".to_string().into(),
            amount: 10000,
            source_currency: "USD".to_string(),
            destination_currency: "HNL".to_string(),
            source_amount: None,
            destination_amount: None,
            exchange_rate: Some(exchange_rate),
            reference: "ref".to_string(),
            purpose: None,
            status: "created".to_string(),
            failure_reason: None,
            sender_details: serde_json::json!({}).into(),
            beneficiary_details: serde_json::json!({}).into(),
            return_url: None,
            metadata: None,
            connector: "test".to_string(),
            client_secret: None,
            remittance_date: date_time::now().date(),
            created_at: None,
            updated_at: None,
        };

        let decimal = remittance.exchange_rate_value();
        assert!(decimal.is_some());
        assert_eq!(decimal.unwrap(), &exchange_rate);
    }
    
    #[test]
    fn test_update_methods() {
        // Probar update builds
        let status_update = RemittanceUpdate::build_status_update("payment_processed");
        if let RemittanceUpdate::StatusUpdate { status, updated_at: _ } = status_update {
            assert_eq!(status, "payment_processed");
        } else {
            panic!("Expected StatusUpdate variant");
        }
        
        let error_update = RemittanceUpdate::build_error_update(
            Some(RemittanceStatus::Failed.to_string()),
            Some("Payment rejected".to_string()),
        );
        if let RemittanceUpdate::ErrorUpdate { status, failure_reason, updated_at: _ } = error_update {
            assert_eq!(status, Some("failed".to_string()));
            assert_eq!(failure_reason, Some("Payment rejected".to_string()));
        } else {
            panic!("Expected ErrorUpdate variant");
        }
        
        let failure_update = RemittanceUpdate::build_failure_update("Connection error");
        if let RemittanceUpdate::Update { status, failure_reason, updated_at: _ } = failure_update {
            assert_eq!(status, "failed");
            assert_eq!(failure_reason, Some("Connection error".to_string()));
        } else {
            panic!("Expected Update variant");
        }
    }
}