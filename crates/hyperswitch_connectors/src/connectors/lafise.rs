// src/connectors/lafise.rs

pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{
        PaymentMethodData, WalletData, Card, ApplePayWalletData, GooglePayWalletData,
    },
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsSyncType, PaymentsVoidType,
        RefundExecuteType, RefundSyncType, Response, SetupMandateType,
    },
    webhooks,
};
use masking::{ExposeInterface, Mask, Maskable, PeekInterface};
use ring::{digest, hmac};
use time::OffsetDateTime;
use transformers as lafise;
use url::Url;

use crate::{
    constants::{self, headers},
    types::ResponseRouterData,
    utils::{self, PaymentMethodDataType, RefundsRequestData},
};

/// Constante con nombre heredado de BankOfAmerica, pero usada para LAFISE.
pub const V_C_MERCHANT_ID: &str = "v-c-merchant-id";

/// Conector LAFISE (similar a BankOfAmerica pero adaptado)
#[derive(Debug, Clone)]
pub struct Lafise;

// Se agrega un método asociado new() para poder construir una instancia de Lafise.
impl Lafise {
    pub fn new() -> Self {
        Lafise
    }
}

// -----------------------------------------------------------------------------
// Funciones Helper para Authorize: convertir Card y WalletData a String
// -----------------------------------------------------------------------------
fn card_to_token(_ccard: &Card) -> String {
    // Implementa aquí la conversión requerida para el flujo Authorize.
    // Por ejemplo, extrae algún identificador o token de la tarjeta.
    // En este ejemplo se retorna un valor dummy.
    "dummy_card_token".to_string()
}

fn apple_pay_to_token(_apple: &ApplePayWalletData) -> String {
    // Implementa la conversión adecuada para ApplePay.
    "dummy_apple_pay_token".to_string()
}

fn google_pay_to_token(_google: &GooglePayWalletData) -> String {
    // Implementa la conversión adecuada para GooglePay.
    "dummy_google_pay_token".to_string()
}

// ======================================================================================
// Implementaciones de traits de Payment, Refund, etc. (API trait stubs)
// ======================================================================================
impl api::Payment for Lafise {}
impl api::PaymentSession for Lafise {}
impl api::ConnectorAccessToken for Lafise {}
impl api::MandateSetup for Lafise {}
impl api::PaymentAuthorize for Lafise {}
impl api::PaymentSync for Lafise {}
impl api::PaymentCapture for Lafise {}
impl api::PaymentVoid for Lafise {}
impl api::Refund for Lafise {}
impl api::RefundExecute for Lafise {}
impl api::RefundSync for Lafise {}
impl api::PaymentToken for Lafise {}

impl Lafise {
    /// Genera un Digest SHA-256 + Base64 sobre el payload (se usa en el header `Digest`)
    pub fn generate_digest(&self, payload: &[u8]) -> String {
        let payload_digest = digest::digest(&digest::SHA256, payload);
        consts::BASE64_ENGINE.encode(payload_digest)
    }

    /// Genera una firma HMAC-SHA256 requerida por la API LAFISE.
    pub fn generate_signature(
        &self,
        auth: lafise::LafiseAuthType,
        host: String,
        resource: &str,
        payload: &String,
        date: OffsetDateTime,
        http_method: Method,
    ) -> CustomResult<String, errors::ConnectorError> {
        let lafise::LafiseAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;

        let is_post_method = matches!(http_method, Method::Post);
        let digest_str = if is_post_method { "digest " } else { "" };
        let headers = format!("host date (request-target) {digest_str}{V_C_MERCHANT_ID}");

        // Ejemplo 'post {resource}' en request-target.
        let request_target = if is_post_method {
            format!("(request-target): post {resource}\ndigest: SHA-256={payload}\n")
        } else {
            format!("(request-target): get {resource}\n")
        };

        let signature_string = format!(
            "host: {host}\ndate: {date}\n{request_target}{V_C_MERCHANT_ID}: {}",
            merchant_account.peek()
        );

        // Se asume que api_secret está en Base64
        let key_value = consts::BASE64_ENGINE
            .decode(api_secret.expose())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "connector_account_details.api_secret",
            })?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, &key_value);
        let signature_value =
            consts::BASE64_ENGINE.encode(hmac::sign(&key, signature_string.as_bytes()).as_ref());

        let signature_header = format!(
            r#"keyid="{}", algorithm="HmacSHA256", headers="{headers}", signature="{signature_value}""#,
            api_key.peek()
        );
        Ok(signature_header)
    }
}

// ======================================================================================
// PaymentMethodToken - sin implementar
// ======================================================================================
impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Lafise
{
    // Not Implemented (R)
}

// ======================================================================================
// Conector - Implementación Common/Validation
// ======================================================================================
impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Lafise
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    /// Construye los headers con HMAC (Signature), date, host, etc.
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let date = OffsetDateTime::now_utc();
        let body_req = self.get_request_body(req, connectors)?;
        let http_method = self.get_http_method();
        let auth = lafise::LafiseAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account = auth.merchant_account.clone();

        let base_url = connectors.lafise.base_url.as_str();
        let lafise_host =
            Url::parse(base_url).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let host = lafise_host
            .host_str()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;

        // path = lo que queda luego de base_url
        let path: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(base_url.len() - 1)
            .collect();

        let sha256 = self.generate_digest(body_req.get_inner_value().expose().as_bytes());
        let signature = self.generate_signature(
            auth,
            host.to_string(),
            path.as_str(),
            &sha256,
            date,
            http_method,
        )?;

        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                "application/hal+json;charset=utf-8".to_string().into(),
            ),
            (
                V_C_MERCHANT_ID.to_string(),
                merchant_account.into_masked(),
            ),
            ("Date".to_string(), date.to_string().into()),
            ("Host".to_string(), host.to_string().into()),
            ("Signature".to_string(), signature.into_masked()),
        ];

        if matches!(http_method, Method::Post | Method::Put) {
            headers.push((
                "Digest".to_string(),
                format!("SHA-256={sha256}").into_masked(),
            ));
        }
        Ok(headers)
    }
}

impl ConnectorCommon for Lafise {
    fn id(&self) -> &'static str {
        "lafise"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json;charset=utf-8"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.lafise.base_url.as_ref()
    }

    /// Crea un ErrorResponse genérico a partir del JSON de error de LAFISE
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: lafise::LafiseErrorResponse = res
            .response
            .parse_struct("lafise ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_message = if res.status_code == 401 {
            constants::CONNECTOR_UNAUTHORIZED_ERROR
        } else {
            hyperswitch_interfaces::consts::NO_ERROR_MESSAGE
        };

        match response {
            lafise::LafiseErrorResponse::StandardError(resp_std) => {
                // Error "estándar"
                let (code, message, reason) = match resp_std.error_information {
                    Some(ref error_info) => {
                        let detailed_error_info = error_info.details.as_ref().map(|details| {
                            details
                                .iter()
                                .map(|det| format!("{} : {}", det.field, det.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });
                        (
                            error_info.reason.clone(),
                            error_info.reason.clone(),
                            lafise::get_error_reason(
                                Some(error_info.message.clone()),
                                detailed_error_info,
                                None,
                            ),
                        )
                    }
                    None => {
                        let detailed_error_info = resp_std.details.map(|details| {
                            details
                                .iter()
                                .map(|det| format!("{} : {}", det.field, det.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });
                        (
                            resp_std
                                .reason
                                .clone()
                                .map_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(), |r| r.to_string()),
                            resp_std
                                .reason
                                .map_or(error_message.to_string(), |r| r.to_string()),
                            lafise::get_error_reason(resp_std.message, detailed_error_info, None),
                        )
                    }
                };

Ok(ErrorResponse {
    status_code: res.status_code,
    code,
    message,
    reason,
    attempt_status: None,
    connector_transaction_id: None,
    network_advice_code: None,
    network_decline_code: None,
    network_error_message: None,
})

            }
            lafise::LafiseErrorResponse::AuthenticationError(resp_auth) => {
                // Error de autenticación
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                    message: resp_auth.response.rmsg.clone(),
                    reason: Some(resp_auth.response.rmsg),
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
network_decline_code: None,
network_error_message: None,

                })
            }
        }
    }
}

// ======================================================================================
// Validaciones del conector
// ======================================================================================
impl ConnectorValidation for Lafise {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic
            | enums::CaptureMethod::Manual
            | enums::CaptureMethod::SequentialAutomatic => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => {
                Err(utils::construct_not_implemented_error_report(capture_method, self.id()))
            }
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::ApplePay,
            PaymentMethodDataType::GooglePay,
        ]);
        utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

// ======================================================================================
// PaymentSession: no implementado
// ======================================================================================
impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Lafise {
    // Not implemented
}

// ======================================================================================
// AccessTokenAuth: no implementado
// ======================================================================================
impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Lafise {
    // Not implemented
}

// ======================================================================================
// SetupMandate (POST /obl/v2/payments) → Ejemplo
// ======================================================================================
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Lafise {
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Ejemplo: POST /obl/v2/payments
        Ok(format!("{}obl/v2/payments/", self.base_url(connectors)))
    }

    /// Aquí se hace el match real según el PaymentMethodData,
    /// llamando a la impl de TryFrom para Card / ApplePay / GooglePay (u otros).
    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        match &req.request.payment_method_data {
            PaymentMethodData::Card(ccard) => {
                // Usa la impl: TryFrom<(&SetupMandateRouterData, Card)>
                let connector_req = lafise::LafisePaymentsRequest::try_from((req, ccard.clone()))?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            PaymentMethodData::Wallet(WalletData::ApplePay(apple_data)) => {
                // Usa la impl: TryFrom<(&SetupMandateRouterData, ApplePayWalletData)>
                let connector_req =
                    lafise::LafisePaymentsRequest::try_from((req, apple_data.clone()))?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            PaymentMethodData::Wallet(WalletData::GooglePay(gpay_data)) => {
                // Usa la impl: TryFrom<(&SetupMandateRouterData, GooglePayWalletData)>
                let connector_req =
                    lafise::LafisePaymentsRequest::try_from((req, gpay_data.clone()))?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "PaymentMethod no soportado en LAFISE SetupMandate".to_string(),
            )
            .into()),
        }
    }

    fn build_request(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&SetupMandateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(SetupMandateType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, errors::ConnectorError> {
        let response: lafise::LafiseSetupMandatesResponse = res
            .response
            .parse_struct("lafiseSetupMandatesResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: lafise::LafiseServerErrorResponse = res
            .response
            .parse_struct("lafiseServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(lafise::Reason::SystemError) => Some(enums::AttemptStatus::Failure),
            Some(lafise::Reason::ServerTimeout) | Some(lafise::Reason::ServiceTimeout) => None,
            None => None,
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            network_advice_code: None,
network_decline_code: None,
network_error_message: None,

        })
    }
}

// ======================================================================================
// Authorize - POST /obl/v2/payments
// ======================================================================================
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Lafise {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}obl/v2/payments/", self.base_url(connectors)))
    }

    /// Aquí también hacemos un match real, para saber si es Card / ApplePay / GooglePay, etc.
    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        // 1) Convierte a un LafiseRouterData (para monto, currency, etc.)
        let connector_router_data = lafise::LafiseRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;

        // 2) match sobre PaymentMethodData para construir LafisePaymentsRequest:
        match &req.request.payment_method_data {
            PaymentMethodData::Card(ccard) => {
                let card_token: String = card_to_token(ccard);
                let connector_req = lafise::LafisePaymentsRequest::try_from((&connector_router_data, card_token))
                    .unwrap_or_else(|_| unreachable!());
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            PaymentMethodData::Wallet(WalletData::ApplePay(apple_data)) => {
                let apple_token: String = apple_pay_to_token(apple_data);
                let connector_req = lafise::LafisePaymentsRequest::try_from((&connector_router_data, apple_token))
                    .unwrap_or_else(|_| unreachable!());
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            PaymentMethodData::Wallet(WalletData::GooglePay(gpay_data)) => {
                let google_token: String = google_pay_to_token(gpay_data);
                let connector_req = lafise::LafisePaymentsRequest::try_from((&connector_router_data, google_token))
                    .unwrap_or_else(|_| unreachable!());
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "PaymentMethod no soportado en LAFISE Authorize".to_string(),
            )
            .into()),
        }
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .set_body(PaymentsAuthorizeType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: lafise::LafisePaymentsResponse = res
            .response
            .parse_struct("lafise PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: lafise::LafiseServerErrorResponse = res
            .response
            .parse_struct("lafiseServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(lafise::Reason::SystemError) => Some(enums::AttemptStatus::Failure),
            Some(lafise::Reason::ServerTimeout) | Some(lafise::Reason::ServiceTimeout) => None,
            None => None,
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            network_advice_code: None,
network_decline_code: None,
network_error_message: None,

        })
    }
}

// ======================================================================================
// PaymentSync - GET /obl/v2/transactions/{connector_payment_id}
// ======================================================================================
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Lafise {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}obl/v2/transactions/{connector_payment_id}",
            self.base_url(connectors)
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: lafise::LafiseTransactionResponse = res
            .response
            .parse_struct("lafise PaymentSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

// ======================================================================================
// Capture: POST /obl/v2/payments/{connector_payment_id}/captures
// ======================================================================================
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Lafise {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}obl/v2/payments/{connector_payment_id}/captures",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = lafise::LafiseRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let connector_req = lafise::LafiseCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsCaptureType::get_headers(self, req, connectors)?)
                .set_body(PaymentsCaptureType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: lafise::LafisePaymentsResponse = res
            .response
            .parse_struct("lafise PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: lafise::LafiseServerErrorResponse = res
            .response
            .parse_struct("lafiseServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
network_decline_code: None,
network_error_message: None,

        })
    }
}

// ======================================================================================
// Void (POST /obl/v2/payments/{connector_payment_id}/reversals)
// ======================================================================================
impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Lafise {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}obl/v2/payments/{connector_payment_id}/reversals",
            self.base_url(connectors)
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = lafise::LafiseRouterData::try_from((
            &self.get_currency_unit(),
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Currency",
                })?,
            req.request
                .amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Amount",
                })?,
            req,
        ))?;
        let connector_req = lafise::LafiseVoidRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(PaymentsVoidType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: lafise::LafisePaymentsResponse = res
            .response
            .parse_struct("lafise PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: lafise::LafiseServerErrorResponse = res
            .response
            .parse_struct("lafiseServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
network_decline_code: None,
network_error_message: None,

        })
    }
}

// ======================================================================================
// Refund: POST /obl/v2/payments/{connector_payment_id}/refunds
// ======================================================================================
impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Lafise {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}obl/v2/payments/{connector_payment_id}/refunds",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = lafise::LafiseRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req = lafise::LafiseRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(RefundExecuteType::get_headers(self, req, connectors)?)
            .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: lafise::LafiseRefundResponse = res
            .response
            .parse_struct("lafise RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

// ======================================================================================
// RefundSync: GET /obl/v2/transactions/{refund_id}
// ======================================================================================
impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Lafise {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}obl/v2/transactions/{refund_id}",
            self.base_url(connectors)
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: lafise::LafiseRsyncResponse = res
            .response
            .parse_struct("lafise RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|ev| ev.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

// ======================================================================================
// Webhooks - no implementado
// ======================================================================================
#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Lafise {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

// ======================================================================================
// Especificaciones del conector (marker trait vacíos).
// ======================================================================================
impl ConnectorSpecifications for Lafise {}