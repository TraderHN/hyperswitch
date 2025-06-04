//! Remittances service layer
//! 
//! This module provides the service abstraction for remittances,
//! handling connector integrations and business logic orchestration

use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, Encode},
    request::{Method, Request, RequestBuilder},
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Mask, Secret};
use router_env::{instrument, tracing};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self as connector_utils, RefundsRequestData},
    core::{
        errors::{self, ConnectorError, RouterResult},
        payments,
    },
    routes::metrics,
    services::{
        self,
        request::{self, Maskable},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        domain,
        ErrorResponse, Response, RouterData,
    },
    AppState,
};

// Import hyperswitch_interfaces types
use hyperswitch_interfaces::api::remittances::{
    RemittanceQuoteOp, RemittanceCreateOp, RemittanceStatusOp, 
    RemittanceCancelOp, RemittancePayoutOp, RemittanceExecuteOp,
    RemittanceQuoteRequestData, RemittanceQuoteResponseData,
    RemittanceCreateRequestData, RemittanceCreateResponseData,
    RemittanceStatusRequestData, RemittanceStatusResponseData,
    RemittanceCancelRequestData, RemittanceCancelResponseData,
    RemittancePayoutRequestData, RemittancePayoutResponseData,
    RemittanceExecuteRequestData, RemittanceExecuteResponseData,
};

/// Trait for remittance connector integration
#[async_trait]
pub trait RemittanceConnectorIntegration:
    ConnectorIntegration<RemittanceQuoteOp, RemittanceQuoteRequestData, RemittanceQuoteResponseData>
    + ConnectorIntegration<RemittanceCreateOp, RemittanceCreateRequestData, RemittanceCreateResponseData>
    + ConnectorIntegration<RemittanceStatusOp, RemittanceStatusRequestData, RemittanceStatusResponseData>
    + ConnectorIntegration<RemittanceCancelOp, RemittanceCancelRequestData, RemittanceCancelResponseData>
    + ConnectorIntegration<RemittancePayoutOp, RemittancePayoutRequestData, RemittancePayoutResponseData>
    + ConnectorCommonExt<RemittanceQuoteOp, RemittanceQuoteRequestData, RemittanceQuoteResponseData>
    + ConnectorCommonExt<RemittanceCreateOp, RemittanceCreateRequestData, RemittanceCreateResponseData>
    + ConnectorCommonExt<RemittanceStatusOp, RemittanceStatusRequestData, RemittanceStatusResponseData>
    + ConnectorCommonExt<RemittanceCancelOp, RemittanceCancelRequestData, RemittanceCancelResponseData>
    + ConnectorCommonExt<RemittancePayoutOp, RemittancePayoutRequestData, RemittancePayoutResponseData>
{
}

/// Remittance flow trait for connector operations
#[async_trait]
pub trait RemittanceFlow:
    ConnectorIntegration<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>
{
}

/// Execute remittance through connector
#[instrument(skip_all)]
pub async fn execute_connector_processing_step<F, Req, Resp, T>(
    state: &AppState,
    connector_integration: T,
    req: &RouterData<F, Req, Resp>,
    call_connector_action: payments::CallConnectorAction,
    connector: &api::ConnectorData,
) -> RouterResult<RouterData<F, Req, Resp>>
where
    T: ConnectorIntegration<F, Req, Resp> + Clone + Send + Sync,
    RouterData<F, Req, Resp>: Feature<F, T>,
    F: Clone + Send + Sync,
    Req: Clone + Send + Sync,
    Resp: Clone + Send + Sync,
{
    let mut router_data = req.clone();
    
    match call_connector_action {
        payments::CallConnectorAction::Trigger => {
            let connector_request = connector_integration
                .build_request(&router_data, &state.conf.connectors)
                .await?;
                
            let response = services::execute_connector_request(
                state,
                connector_integration.clone(),
                &router_data,
                connector_request,
            )
            .await?;
            
            router_data = connector_integration
                .handle_response(&router_data, response)
                .await?;
        }
        
        _ => {
            router_data.response = Err(ErrorResponse {
                code: "IR_00".to_string(),
                message: "Invalid connector action".to_string(),
                reason: None,
                status_code: 500,
                attempt_status: None,
                connector_transaction_id: None,
            });
        }
    }
    
    Ok(router_data)
}

/// Execute a remittance operation through the appropriate connector
#[instrument(skip_all)]
pub async fn execute_remittance_operation<Op>(
    state: &AppState,
    connector: api::ConnectorData,
    operation: Op,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req_data: Op::Request,
) -> RouterResult<Op::Response>
where
    Op: RemittanceOperation + Send + Sync + Clone,
    Op::Request: Send + Sync,
    Op::Response: Send + Sync,
{
    let connector_id = connector.connector.id();
    
    metrics::CONNECTOR_CALL_COUNT.add(
        &metrics::CONTEXT,
        1,
        &[
            metrics::KeyValue::new("connector", connector_id),
            metrics::KeyValue::new("flow", Op::get_operation_name()),
        ],
    );
    
    let router_data = Op::construct_router_data(
        state,
        connector.clone(),
        merchant_account,
        key_store,
        req_data,
    )
    .await?;
    
    let response = match connector.connector {
        #[cfg(feature = "connector_wise")]
        api_models::enums::Connector::Wise => {
            execute_connector_processing_step(
                state,
                hyperswitch_connectors::connectors::Wise,
                &router_data,
                payments::CallConnectorAction::Trigger,
                &connector,
            )
            .await
        }
        
        #[cfg(feature = "connector_currencycloud")]
        api_models::enums::Connector::Currencycloud => {
            execute_connector_processing_step(
                state,
                hyperswitch_connectors::connectors::Currencycloud,
                &router_data,
                payments::CallConnectorAction::Trigger,
                &connector,
            )
            .await
        }
        
        _ => Err(errors::ConnectorError::NotImplemented(
            connector_utils::get_unimplemented_payment_method_error_message("remittances", connector_id),
        )
        .into()),
    }?;
    
    Op::extract_response(response)
}

/// Trait for remittance operations
#[async_trait]
pub trait RemittanceOperation: Sized {
    type Request: Send + Sync;
    type Response: Send + Sync;
    
    fn get_operation_name() -> &'static str;
    
    async fn construct_router_data(
        state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        request: Self::Request,
    ) -> RouterResult<RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>>;
    
    fn extract_response(
        router_data: RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response>;
}

/// Quote operation
#[derive(Debug, Clone)]
pub struct GetQuote;

#[async_trait]
impl RemittanceOperation for GetQuote {
    type Request = api_models::remittances::RemittanceQuoteRequest;
    type Response = api_models::remittances::RemittanceQuoteResponse;
    
    fn get_operation_name() -> &'static str {
        "remittance_quote"
    }
    
    async fn construct_router_data(
        _state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        request: Self::Request,
    ) -> RouterResult<RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>> {
        let auth_type = connector_utils::get_auth_type(&connector.connector_auth_type)?;
        
        Ok(RouterData {
            flow: std::marker::PhantomData,
            merchant_id: merchant_account.merchant_id.clone(),
            customer_id: None,
            attempt_id: format!("quote_{}", uuid::Uuid::new_v4()),
            status: common_enums::AttemptStatus::Started,
            payment_method: common_enums::PaymentMethod::BankTransfer,
            connector: connector.connector.to_string(),
            auth_type,
            description: Some("Get remittance quote".to_string()),
            return_url: None,
            address: types::PaymentAddress::default(),
            connector_meta_data: connector.connector_meta_data,
            connector_wallets_details: connector.connector_wallets_details,
            request: RemittanceExecuteRequestData::Quote(RemittanceQuoteRequestData {
                source_currency: request.source_currency,
                destination_currency: request.destination_currency,
                source_amount: request.amount.get_amount_as_i64(),
                source_country: request.source_country,
                destination_country: request.destination_country,
            }),
            response: Ok(RemittanceExecuteResponseData::Quote(RemittanceQuoteResponseData::default())),
            amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_status: None,
            connector_customer: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_api_version: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            refund_id: None,
            dispute_id: None,
            connector_response: None,
            integrity_check: Ok(()),
        })
    }
    
    fn extract_response(
        router_data: RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response> {
        match router_data.response? {
            RemittanceExecuteResponseData::Quote(quote_data) => {
                Ok(api_models::remittances::RemittanceQuoteResponse {
                    source_currency: quote_data.source_currency,
                    destination_currency: quote_data.destination_currency,
                    source_amount: common_utils::types::MinorUnit::new(quote_data.source_amount),
                    destination_amount: common_utils::types::MinorUnit::new(quote_data.destination_amount),
                    rate: quote_data.rate,
                    fee: quote_data.fee.map(common_utils::types::MinorUnit::new),
                    total_cost: common_utils::types::MinorUnit::new(
                        quote_data.source_amount + quote_data.fee.unwrap_or(0)
                    ),
                    estimated_delivery_time: quote_data.estimated_delivery_time,
                    rate_valid_until: quote_data.rate_valid_until,
                    connector: router_data.connector,
                })
            }
            _ => Err(errors::ApiErrorResponse::InternalServerError.into()),
        }
    }
}

/// Create remittance operation
#[derive(Debug, Clone)]
pub struct CreateRemittance;

#[async_trait]
impl RemittanceOperation for CreateRemittance {
    type Request = (api_models::remittances::RemittanceRequest, String); // (request, remittance_id)
    type Response = api_models::remittances::RemittanceResponse;
    
    fn get_operation_name() -> &'static str {
        "remittance_create"
    }
    
    async fn construct_router_data(
        _state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        (request, remittance_id): Self::Request,
    ) -> RouterResult<RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>> {
        let auth_type = connector_utils::get_auth_type(&connector.connector_auth_type)?;
        
        Ok(RouterData {
            flow: std::marker::PhantomData,
            merchant_id: merchant_account.merchant_id.clone(),
            customer_id: request.sender_details.customer_id.clone(),
            attempt_id: remittance_id.clone(),
            status: common_enums::AttemptStatus::Started,
            payment_method: common_enums::PaymentMethod::BankTransfer,
            connector: connector.connector.to_string(),
            auth_type,
            description: Some(request.reference.clone()),
            return_url: request.return_url.clone().and_then(|u| url::Url::parse(&u).ok()),
            address: types::PaymentAddress::default(),
            connector_meta_data: connector.connector_meta_data,
            connector_wallets_details: connector.connector_wallets_details,
            request: RemittanceExecuteRequestData::Create(RemittanceCreateRequestData {
                remittance_id: remittance_id.clone(),
                source_currency: request.source_currency,
                destination_currency: request.destination_currency,
                source_amount: request.amount.get_amount_as_i64(),
                destination_amount: None, // Will be calculated by connector
                sender_details: request.sender_details,
                beneficiary_details: request.beneficiary_details,
                purpose: request.purpose,
                reference: request.reference,
                metadata: request.metadata,
            }),
            response: Ok(RemittanceExecuteResponseData::Create(RemittanceCreateResponseData::default())),
            amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: Some(remittance_id),
            payment_method_status: None,
            connector_customer: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_api_version: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            refund_id: None,
            dispute_id: None,
            connector_response: None,
            integrity_check: Ok(()),
        })
    }
    
    fn extract_response(
        router_data: RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response> {
        match router_data.response? {
            RemittanceExecuteResponseData::Create(create_data) => {
                // Transform connector response to API response
                Ok(api_models::remittances::RemittanceResponse {
                    remittance_id: create_data.remittance_id,
                    merchant_id: router_data.merchant_id,
                    profile_id: common_utils::id_type::ProfileId::default(), // TODO: Get from request
                    amount: common_utils::types::MinorUnit::new(create_data.source_amount),
                    source_currency: create_data.source_currency,
                    destination_currency: create_data.destination_currency,
                    source_amount: common_utils::types::MinorUnit::new(create_data.source_amount),
                    destination_amount: common_utils::types::MinorUnit::new(
                        create_data.destination_amount.unwrap_or(0)
                    ),
                    exchange_rate: create_data.exchange_rate_info,
                    sender_details: Some(create_data.sender_details),
                    beneficiary_details: Some(create_data.beneficiary_details),
                    remittance_date: create_data.remittance_date,
                    reference: create_data.reference,
                    purpose: create_data.purpose,
                    status: create_data.status,
                    failure_reason: create_data.failure_reason,
                    return_url: create_data.return_url,
                    metadata: create_data.metadata,
                    connector: router_data.connector,
                    client_secret: create_data.client_secret,
                    payment_id: None,
                    payout_id: None,
                    payment_connector_transaction_id: None,
                    payout_connector_transaction_id: None,
                    compliance_status: None,
                    required_documents: None,
                    estimated_delivery_time: create_data.estimated_delivery_time,
                    actual_delivery_time: None,
                    created_at: Some(common_utils::date_time::now()),
                    updated_at: Some(common_utils::date_time::now()),
                })
            }
            _ => Err(errors::ApiErrorResponse::InternalServerError.into()),
        }
    }
}

/// Check remittance status operation
#[derive(Debug, Clone)]
pub struct CheckStatus;

#[async_trait]
impl RemittanceOperation for CheckStatus {
    type Request = String; // remittance_id
    type Response = RemittanceStatusResponseData;
    
    fn get_operation_name() -> &'static str {
        "remittance_status"
    }
    
    async fn construct_router_data(
        _state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        remittance_id: Self::Request,
    ) -> RouterResult<RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>> {
        let auth_type = connector_utils::get_auth_type(&connector.connector_auth_type)?;
        
        Ok(RouterData {
            flow: std::marker::PhantomData,
            merchant_id: merchant_account.merchant_id.clone(),
            customer_id: None,
            attempt_id: remittance_id.clone(),
            status: common_enums::AttemptStatus::Started,
            payment_method: common_enums::PaymentMethod::BankTransfer,
            connector: connector.connector.to_string(),
            auth_type,
            description: Some("Check remittance status".to_string()),
            return_url: None,
            address: types::PaymentAddress::default(),
            connector_meta_data: connector.connector_meta_data,
            connector_wallets_details: connector.connector_wallets_details,
            request: RemittanceExecuteRequestData::Status(RemittanceStatusRequestData {
                remittance_id: remittance_id.clone(),
                connector_remittance_id: None, // Will be looked up
            }),
            response: Ok(RemittanceExecuteResponseData::Status(RemittanceStatusResponseData::default())),
            amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: Some(remittance_id),
            payment_method_status: None,
            connector_customer: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_api_version: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            refund_id: None,
            dispute_id: None,
            connector_response: None,
            integrity_check: Ok(()),
        })
    }
    
    fn extract_response(
        router_data: RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response> {
        match router_data.response? {
            RemittanceExecuteResponseData::Status(status_data) => Ok(status_data),
            _ => Err(errors::ApiErrorResponse::InternalServerError.into()),
        }
    }
}

/// Cancel remittance operation
#[derive(Debug, Clone)]
pub struct CancelRemittance;

#[async_trait]
impl RemittanceOperation for CancelRemittance {
    type Request = (String, String); // (remittance_id, reason)
    type Response = RemittanceCancelResponseData;
    
    fn get_operation_name() -> &'static str {
        "remittance_cancel"
    }
    
    async fn construct_router_data(
        _state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        (remittance_id, reason): Self::Request,
    ) -> RouterResult<RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>> {
        let auth_type = connector_utils::get_auth_type(&connector.connector_auth_type)?;
        
        Ok(RouterData {
            flow: std::marker::PhantomData,
            merchant_id: merchant_account.merchant_id.clone(),
            customer_id: None,
            attempt_id: remittance_id.clone(),
            status: common_enums::AttemptStatus::Started,
            payment_method: common_enums::PaymentMethod::BankTransfer,
            connector: connector.connector.to_string(),
            auth_type,
            description: Some("Cancel remittance".to_string()),
            return_url: None,
            address: types::PaymentAddress::default(),
            connector_meta_data: connector.connector_meta_data,
            connector_wallets_details: connector.connector_wallets_details,
            request: RemittanceExecuteRequestData::Cancel(RemittanceCancelRequestData {
                remittance_id: remittance_id.clone(),
                connector_remittance_id: None, // Will be looked up
                reason,
            }),
            response: Ok(RemittanceExecuteResponseData::Cancel(RemittanceCancelResponseData::default())),
            amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: Some(remittance_id),
            payment_method_status: None,
            connector_customer: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_api_version: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            refund_id: None,
            dispute_id: None,
            connector_response: None,
            integrity_check: Ok(()),
        })
    }
    
    fn extract_response(
        router_data: RouterData<RemittanceExecuteOp, RemittanceExecuteRequestData, RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response> {
        match router_data.response? {
            RemittanceExecuteResponseData::Cancel(cancel_data) => Ok(cancel_data),
            _ => Err(errors::ApiErrorResponse::InternalServerError.into()),
        }
    }
}

/// Feature trait for router data
pub trait Feature<F, T>: Sized {
    fn get_flow(&self) -> F;
}

impl<F, Req, Resp, T> Feature<F, T> for RouterData<F, Req, Resp>
where
    F: Clone,
    T: ConnectorIntegration<F, Req, Resp>,
{
    fn get_flow(&self) -> F {
        self.flow.clone()
    }
}

/// Helper function to execute connector request
async fn execute_connector_request<F, Req, Resp, T>(
    state: &AppState,
    connector_integration: T,
    router_data: &RouterData<F, Req, Resp>,
    connector_request: Request<Req>,
) -> RouterResult<Response<Resp>>
where
    T: ConnectorIntegration<F, Req, Resp> + Send + Sync,
    F: Send + Sync,
    Req: Send + Sync,
    Resp: Send + Sync,
{
    // Execute the actual HTTP request to the connector
    let response = services::call_connector_api(state, connector_request).await?;
    Ok(response)
}

impl Default for RemittanceQuoteResponseData {
    fn default() -> Self {
        Self {
            source_currency: common_enums::Currency::USD,
            destination_currency: common_enums::Currency::USD,
            source_amount: 0,
            destination_amount: 0,
            rate: 1.0,
            fee: None,
            estimated_delivery_time: None,
            rate_valid_until: None,
        }
    }
}

impl Default for RemittanceCreateResponseData {
    fn default() -> Self {
        Self {
            remittance_id: String::new(),
            source_currency: common_enums::Currency::USD,
            destination_currency: common_enums::Currency::USD,
            source_amount: 0,
            destination_amount: None,
            exchange_rate_info: None,
            sender_details: api_models::remittances::SenderDetails::default(),
            beneficiary_details: api_models::remittances::BeneficiaryDetails::default(),
            remittance_date: String::new(),
            reference: String::new(),
            purpose: None,
            status: api_models::remittances::RemittanceStatus::Created,
            failure_reason: None,
            return_url: None,
            metadata: None,
            client_secret: None,
            estimated_delivery_time: None,
        }
    }
}

impl Default for RemittanceStatusResponseData {
    fn default() -> Self {
        Self {
            status: api_models::remittances::RemittanceStatus::Created,
            connector_transaction_id: None,
            payment_status: None,
            payout_status: None,
        }
    }
}

impl Default for RemittanceCancelResponseData {
    fn default() -> Self {
        Self {
            status: api_models::remittances::RemittanceStatus::Cancelled,
            cancelled_at: None,
        }
    }
}