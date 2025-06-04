// crates/hyperswitch_interfaces/src/api/remittances.rs

//! Remittances interface
//!
//! Define los tipos de mensajes y traits que los conectores de remesas deben implementar.

use super::ConnectorCommon;
use crate::api::ConnectorIntegration;

use serde::{Deserialize, Serialize};
use api_models::remittances::{
    RemittanceQuoteRequest, RemittanceQuoteResponse,
    RemittanceRequest, RemittanceResponse,
    RemittancePayRequest,
};
use api_models::remittances::RemittanceStatus as ApiRemittanceStatus;

// -----------------------------------------------------------------------------
// Identificadores de operación
// -----------------------------------------------------------------------------

/// Operación de cotización de tipo de cambio.
#[derive(Debug, Clone)]
pub struct RemittanceQuoteOp;

/// Operación de creación de una nueva remesa.
#[derive(Debug, Clone)]
pub struct RemittanceCreateOp;

/// Operación de consulta de estado de remesa.
#[derive(Debug, Clone)]
pub struct RemittanceStatusOp;

/// Operación de cancelación de remesa.
#[derive(Debug, Clone)]
pub struct RemittanceCancelOp;

/// Operación de liquidación (payout) de remesa.
#[derive(Debug, Clone)]
pub struct RemittancePayoutOp;

/// Operación para ejecutar un flujo completo de remesa.
#[derive(Debug, Clone)]
pub struct RemittanceExecuteOp;

// -----------------------------------------------------------------------------
// Reuso de modelos de API para request/response
// -----------------------------------------------------------------------------

/// Alias para los datos de request de cotización de remesas.
pub type RemittanceQuoteRequestData = RemittanceQuoteRequest;
/// Alias para los datos de response de cotización de remesas.
pub type RemittanceQuoteResponseData = RemittanceQuoteResponse;

/// Alias para los datos de request de creación de remesa.
pub type RemittanceCreateRequestData = RemittanceRequest;
/// Alias para los datos de response de creación de remesa.
pub type RemittanceCreateResponseData = RemittanceResponse;

/// Alias para los datos de request de payout de remesa.
pub type RemittancePayoutRequestData = RemittancePayRequest;
/// Alias para los datos de response de payout de remesa.
pub type RemittancePayoutResponseData = RemittanceResponse;

// -----------------------------------------------------------------------------
// Tipos auxiliares para status y cancel
// -----------------------------------------------------------------------------

/// Request para consultar el estado de una remesa.
///
/// - `remittance_id`: ID interno de la remesa.
/// - `connector_remittance_id`: ID de remesa en el conector externo (opcional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceStatusRequestData {
    /// ID interno de la remesa en nuestra plataforma.
    pub remittance_id: String,
    /// ID de la remesa en el sistema del conector (si existe).
    pub connector_remittance_id: Option<String>,
}

/// Response al consultar el estado de una remesa.
///
/// - `status`: estado actual de la remesa.
/// - `connector_transaction_id`: ID de la transacción en el conector (opcional).
/// - `payment_status`: estado interno de pago (opcional).
/// - `payout_status`: estado interno de liquidación (opcional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceStatusResponseData {
    /// Estado de la remesa.
    pub status: ApiRemittanceStatus,
    /// ID de la transacción asignado por el conector.
    pub connector_transaction_id: Option<String>,
    /// Estado interno del paso de pago.
    pub payment_status: Option<String>,
    /// Estado interno del paso de liquidación.
    pub payout_status: Option<String>,
}

/// Request para cancelar una remesa.
///
/// - `remittance_id`: ID interno de la remesa.
/// - `connector_remittance_id`: ID de remesa en el conector (opcional).
/// - `reason`: motivo de la cancelación.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceCancelRequestData {
    /// ID interno de la remesa en nuestra plataforma.
    pub remittance_id: String,
    /// ID de la remesa en el sistema del conector (si existe).
    pub connector_remittance_id: Option<String>,
    /// Motivo de la cancelación.
    pub reason: String,
}

/// Response tras cancelar una remesa.
///
/// - `status`: estado final de la remesa (debería ser “cancelled”).
/// - `cancelled_at`: timestamp de cancelación en formato ISO 8601 (opcional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceCancelResponseData {
    /// Estado de la remesa después de la cancelación.
    pub status: ApiRemittanceStatus,
    /// Fecha y hora de la cancelación.
    pub cancelled_at: Option<String>,
}

// -----------------------------------------------------------------------------
// Envelope para ejecutar cualquier operación de remesa
// -----------------------------------------------------------------------------

/// Envelope genérico para requests de operaciones de remesas.
///
/// Selecciona la operación mediante el campo `"type"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RemittanceExecuteRequestData {
    /// Cotizar tipo de cambio.
    Quote(RemittanceQuoteRequestData),
    /// Crear nueva remesa.
    Create(RemittanceCreateRequestData),
    /// Consultar estado de remesa.
    Status(RemittanceStatusRequestData),
    /// Cancelar remesa.
    Cancel(RemittanceCancelRequestData),
    /// Procesar liquidación (payout).
    Payout(RemittancePayoutRequestData),
}

/// Envelope genérico para responses de operaciones de remesas.
///
/// El campo `"type"` indica la operación realizada.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RemittanceExecuteResponseData {
    /// Respuesta de cotización.
    Quote(RemittanceQuoteResponseData),
    /// Respuesta de creación.
    Create(RemittanceCreateResponseData),
    /// Respuesta de estado.
    Status(RemittanceStatusResponseData),
    /// Respuesta de cancelación.
    Cancel(RemittanceCancelResponseData),
    /// Respuesta de liquidación (payout).
    Payout(RemittancePayoutResponseData),
}

// -----------------------------------------------------------------------------
// Traits que deben implementar los conectores de remesas
// -----------------------------------------------------------------------------

/// Trait para conectar la operación de cotización de remesas.
pub trait RemittanceQuoteConnector:
    ConnectorIntegration<RemittanceQuoteOp, RemittanceQuoteRequestData, RemittanceQuoteResponseData>
{
}

/// Trait para conectar la operación de creación de remesas.
pub trait RemittanceCreateConnector:
    ConnectorIntegration<RemittanceCreateOp, RemittanceCreateRequestData, RemittanceCreateResponseData>
{
}

/// Trait para conectar la operación de consulta de estado de remesas.
pub trait RemittanceStatusConnector:
    ConnectorIntegration<RemittanceStatusOp, RemittanceStatusRequestData, RemittanceStatusResponseData>
{
}

/// Trait para conectar la operación de cancelación de remesas.
pub trait RemittanceCancelConnector:
    ConnectorIntegration<RemittanceCancelOp, RemittanceCancelRequestData, RemittanceCancelResponseData>
{
}

/// Trait para conectar la operación de liquidación (payout) de remesas.
pub trait RemittancePayoutConnector:
    ConnectorIntegration<RemittancePayoutOp, RemittancePayoutRequestData, RemittancePayoutResponseData>
{
}

/// Trait para ejecutar cualquier operación de remesas mediante un envelope.
pub trait RemittanceExecuteConnector:
    ConnectorIntegration<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>
{
}

#[cfg(feature = "remittances")]
/// Trait unificado que agrupa todas las operaciones de remesas.
pub trait Remittances:
    ConnectorCommon
    + RemittanceQuoteConnector
    + RemittanceCreateConnector
    + RemittanceStatusConnector
    + RemittanceCancelConnector
    + RemittancePayoutConnector
    + RemittanceExecuteConnector
{
}

#[cfg(not(feature = "remittances"))]
/// Trait vacío cuando la feature `remittances` está desactivada.
pub trait Remittances {}
