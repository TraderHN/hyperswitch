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

/// Trait for remittance connector integration
#[async_trait]
pub trait RemittanceConnectorIntegration:
    ConnectorIntegration
        api::remittances::RemittanceQuote,
        types::RemittanceQuoteRequestData,
        types::RemittanceQuoteResponseData,
    > + ConnectorIntegration
        api::remittances::RemittanceCreate,
        types::RemittanceCreateRequestData,
        types::RemittanceCreateResponseData,
    > + ConnectorIntegration
        api::remittances::RemittanceStatus,
        types::RemittanceStatusRequestData,
        types::RemittanceStatusResponseData,
    > + ConnectorIntegration
        api::remittances::RemittanceCancel,
        types::RemittanceCancelRequestData,
        types::RemittanceCancelResponseData,
    > + ConnectorIntegration
        api::remittances::RemittancePayout,
        types::RemittancePayoutRequestData,
        types::RemittancePayoutResponseData,
    > + ConnectorCommonExt
        api::remittances::RemittanceQuote,
        types::RemittanceQuoteRequestData,
        types::RemittanceQuoteResponseData,
    > + ConnectorCommonExt
        api::remittances::RemittanceCreate,
        types::RemittanceCreateRequestData,
        types::RemittanceCreateResponseData,
    > + ConnectorCommonExt
        api::remittances::RemittanceStatus,
        types::RemittanceStatusRequestData,
        types::RemittanceStatusResponseData,
    > + ConnectorCommonExt
        api::remittances::RemittanceCancel,
        types::RemittanceCancelRequestData,
        types::RemittanceCancelResponseData,
    > + ConnectorCommonExt
        api::remittances::RemittancePayout,
        types::RemittancePayoutRequestData,
        types::RemittancePayoutResponseData,
    >
{
}

/// Remittance flow trait for connector operations
#[async_trait]
pub trait RemittanceFlow:
    services::ConnectorIntegration
        api::remittances::RemittanceExecute,
        types::RemittanceExecuteRequestData,
        types::RemittanceExecuteResponseData,
    >
{
}

/// Execute remittance through connector
#[instrument(skip_all)]
pub async fn execute_connector_processing_step<'a, F, Req, Resp, T, Ctx>(
    state: &'a AppState,
    connector_integration: T,
    req: &RouterData<F, Req, Resp>,
    call_connector_action: payments::CallConnectorAction,
    connector: &api::ConnectorData,
) -> RouterResult<RouterData<F, Req, Resp>>
where
    T: ConnectorIntegration<F, Req, Resp> + Clone,
    RouterData<F, Req, Resp>: Feature<F, T>,
    F: Clone,
    Req: Clone,
    Resp: Clone,
{
    let mut router_data = req.clone();
    
    match call_connector_action {
        payments::CallConnectorAction::Trigger => {
            let connector_request = connector_integration
                .build_request(&router_data, &state.conf.connectors)
                .await?;
                
            let response = services::execute(
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
    type Request;
    type Response;
    
    fn get_operation_name() -> &'static str;
    
    async fn construct_router_data(
        state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        request: Self::Request,
    ) -> RouterResult<RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>>;
    
    fn extract_response(
        router_data: RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response>;
}

/// Quote operation
#[derive(Debug, Clone)]
pub struct GetQuote;

#[async_trait]
impl RemittanceOperation for GetQuote {
    type Request = api::remittances::RemittanceQuoteRequest;
    type Response = api::remittances::RemittanceQuoteResponse;
    
    fn get_operation_name() -> &'static str {
        "remittance_quote"
    }
    
    async fn construct_router_data(
        _state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        request: Self::Request,
    ) -> RouterResult<RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>> {
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
            request: types::RemittanceExecuteRequestData::Quote(types::RemittanceQuoteRequestData {
                source_currency: request.source_currency,
                destination_currency: request.destination_currency,
                source_amount: request.amount.get_amount(),
                source_country: request.source_country,
                destination_country: request.destination_country,
            }),
            response: Ok(types::RemittanceExecuteResponseData::default()),
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
        router_data: RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response> {
        match router_data.response? {
            types::RemittanceExecuteResponseData::Quote(quote_data) => {
                Ok(api::remittances::RemittanceQuoteResponse {
                    source_currency: quote_data.source_currency,
                    destination_currency: quote_data.destination_currency,
                    source_amount: common_utils::types::MinorUnit::new(quote_data.source_amount),
                    destination_amount: common_utils::types::MinorUnit::new(quote_data.destination_amount),
                    rate: quote_data.exchange_rate,
                    fee: quote_data.fee.map(common_utils::types::MinorUnit::new),
                    total_cost: common_utils::types::MinorUnit::new(
                        quote_data.source_amount + quote_data.fee.unwrap_or(0)
                    ),
                    estimated_delivery_time: quote_data.estimated_delivery_hours,
                    rate_valid_until: quote_data.rate_expiry_time,
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
    type Request = (api::remittances::RemittanceCreateRequest, String); // (request, remittance_id)
    type Response = types::RemittanceCreateResponseData;
    
    fn get_operation_name() -> &'static str {
        "remittance_create"
    }
    
    async fn construct_router_data(
        _state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        (request, remittance_id): Self::Request,
    ) -> RouterResult<RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>> {
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
            request: types::RemittanceExecuteRequestData::Create(types::RemittanceCreateRequestData {
                remittance_id: remittance_id.clone(),
                source_currency: request.source_currency,
                destination_currency: request.destination_currency,
                source_amount: request.amount.get_amount(),
                destination_amount: None, // Will be calculated by connector
                sender: types::RemittanceSenderData {
                    name: request.sender_details.name,
                    address: request.sender_details.address,
                    email: request.sender_details.email,
                    phone: request.sender_details.phone,
                },
                beneficiary: types::RemittanceBeneficiaryData {
                    name: request.beneficiary_details.name,
                    address: request.beneficiary_details.address,
                    email: request.beneficiary_details.email,
                    phone: request.beneficiary_details.phone,
                    account_details: map_payout_method_to_connector(
                        request.beneficiary_details.payout_details
                    )?,
                },
                purpose: request.purpose.map(|p| p.to_string()),
                reference: request.reference,
                metadata: request.metadata,
            }),
            response: Ok(types::RemittanceExecuteResponseData::default()),
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
        router_data: RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response> {
        match router_data.response? {
            types::RemittanceExecuteResponseData::Create(create_data) => Ok(create_data),
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
    type Response = types::RemittanceStatusResponseData;
    
    fn get_operation_name() -> &'static str {
        "remittance_status"
    }
    
    async fn construct_router_data(
        _state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        remittance_id: Self::Request,
    ) -> RouterResult<RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>> {
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
            request: types::RemittanceExecuteRequestData::Status(types::RemittanceStatusRequestData {
                remittance_id: remittance_id.clone(),
                connector_remittance_id: None, // Will be looked up
            }),
            response: Ok(types::RemittanceExecuteResponseData::default()),
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
        router_data: RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response> {
        match router_data.response? {
            types::RemittanceExecuteResponseData::Status(status_data) => Ok(status_data),
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
    type Response = types::RemittanceCancelResponseData;
    
    fn get_operation_name() -> &'static str {
        "remittance_cancel"
    }
    
    async fn construct_router_data(
        _state: &AppState,
        connector: api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        (remittance_id, reason): Self::Request,
    ) -> RouterResult<RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>> {
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
            request: types::RemittanceExecuteRequestData::Cancel(types::RemittanceCancelRequestData {
                remittance_id: remittance_id.clone(),
                connector_remittance_id: None, // Will be looked up
                reason,
            }),
            response: Ok(types::RemittanceExecuteResponseData::default()),
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
        router_data: RouterData<api::remittances::RemittanceExecute, types::RemittanceExecuteRequestData, types::RemittanceExecuteResponseData>,
    ) -> RouterResult<Self::Response> {
        match router_data.response? {
            types::RemittanceExecuteResponseData::Cancel(cancel_data) => Ok(cancel_data),
            _ => Err(errors::ApiErrorResponse::InternalServerError.into()),
        }
    }
}

/// Helper function to map payout method to connector format
fn map_payout_method_to_connector(
    payout_method: Option<api::remittances::PayoutMethodData>,
) -> RouterResult<types::RemittanceAccountDetails> {
    match payout_method {
        Some(api::remittances::PayoutMethodData::BankTransfer(bank_data)) => {
            Ok(types::RemittanceAccountDetails::Bank {
                account_number: bank_data.account_number,
                routing_number: bank_data.routing_number,
                iban: bank_data.iban,
                bic: bank_data.bic,
                bank_name: bank_data.bank_name,
                bank_country: bank_data.bank_country.map(|c| c.to_string()),
            })
        }
        Some(api::remittances::PayoutMethodData::Wallet(wallet_data)) => {
            Ok(types::RemittanceAccountDetails::Wallet {
                wallet_id: wallet_data.wallet_id,
                wallet_type: wallet_data.wallet_type.to_string(),
                provider_details: wallet_data.provider_details,
            })
        }
        _ => Ok(types::RemittanceAccountDetails::Other),
    }
}

/// Feature trait for router data
pub trait Feature<F, T>: Sized {
    fn get_flow(&self) -> F;
}

impl<F, Req, Resp> Feature<F, impl ConnectorIntegration<F, Req, Resp>> for RouterData<F, Req, Resp>
where
    F: Clone,
{
    fn get_flow(&self) -> F {
        self.flow.clone()
    }
}