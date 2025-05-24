//! Routes for remittances
//! This module defines the API endpoints for remittance operations

use actix_web::{web, HttpRequest, HttpResponse};
use common_utils::{
    errors::ReportSwitchExt,
    id_type::{MerchantId, ProfileId},
};
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use crate::{
    app::AppState,
    core::{
        api_keys,
        errors::{self, ApiErrorResponse, RouterResult},
        remittances,
    },
    logger,
    routes::{self, lock_utils, AppStateInfo, IdempotentFlow, LockAction, ReqState, Router},
    services::{
        api::authentication::{ApiKeyAuth, ApiKeyAuthWithMerchantIdFromRoute, HeaderAuth, JWTAuth, JWTAuthMerchantFromRoute, NoAuth, PublishableKeyAuth},
        authorization::permissions::Permission,
    },
    types::api::{
        admin,
        remittances::{
            RemittanceCreateRequest, RemittanceListRequest, RemittancePayRequest, 
            RemittanceRetrieveRequest, RemittanceSyncRequest, RemittanceUpdateRequest
        },
        self, AuthFlow,
    },
    utils::{self, Encode},
};

pub struct RemittanceState<F>
where
    F: Clone,
{
    pub(crate) flow: F,
}

impl<F> RemittanceState<F>
where
    F: Clone,
{
    pub fn new(flow: F) -> Self {
        Self { flow }
    }
}

pub type RemittanceResponse = HttpResponse;

#[derive(Debug, Copy, Clone, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum RemittanceFail {
    InvalidRemittanceId,
    MissingRequiredField,
    InvalidDataFormat,
    RemittanceNotFound,
    RemittanceUpdateForbidden,
    RemittancePaymentForbidden,
    RemittanceCancellationForbidden,
    DuplicateRemittance,
    MerchantAccountNotFound,
    InvalidConnector,
    InternalServerError,
}

impl From<RemittanceFail> for ApiErrorResponse {
    fn from(item: RemittanceFail) -> Self {
        match item {
            RemittanceFail::InvalidRemittanceId => Self::InvalidRequestData {
                message: "Invalid remittance ID".to_string(),
            },
            RemittanceFail::MissingRequiredField => Self::MissingRequiredField {
                field_name: "field",
            },
            RemittanceFail::InvalidDataFormat => Self::InvalidDataFormat {
                field_name: "field".to_string(),
                expected_format: "format".to_string(),
            },
            RemittanceFail::RemittanceNotFound => Self::ResourceNotFound {
                resource: "remittance".to_string(),
            },
            RemittanceFail::RemittanceUpdateForbidden => Self::RemittanceUpdateForbidden,
            RemittanceFail::RemittancePaymentForbidden => Self::RemittancePaymentForbidden,
            RemittanceFail::RemittanceCancellationForbidden => Self::RemittanceCancellationForbidden,
            RemittanceFail::DuplicateRemittance => Self::DuplicateRequest {
                message: "Duplicate remittance".to_string(),
            },
            RemittanceFail::MerchantAccountNotFound => Self::MerchantAccountNotFound,
            RemittanceFail::InvalidConnector => Self::InvalidRequestData {
                message: "Invalid connector".to_string(),
            },
            RemittanceFail::InternalServerError => Self::InternalServerError,
        }
    }
}

pub struct RemittanceRequestInfo {
    pub remittance_id: String,
    pub merchant_id: Option<MerchantId>,
    pub profile_id: Option<ProfileId>,
}

pub enum LockingAction {
    CreateRemittance,
    PayRemittance,
    UpdateRemittance,
    CancelRemittance,
}

impl LockAction for LockingAction {
    fn to_string(&self) -> String {
        match self {
            Self::CreateRemittance => "CreateRemittance".to_string(),
            Self::PayRemittance => "PayRemittance".to_string(),
            Self::UpdateRemittance => "UpdateRemittance".to_string(),
            Self::CancelRemittance => "CancelRemittance".to_string(),
        }
    }
}

/// API endpoints for remittances
pub fn configure_routes<A>(app: A, state: web::Data<AppState>) -> A
where
    A: web::ServiceConfig,
{
    app.app_data(web::Data::new(RemittanceState::new(IdempotentFlow)))
        // Create remittance
        .service(
            web::resource("/remittances")
                .app_data(web::Data::new(ReqState::new("")))
                .route(
                    web::post().to(create_remittance::<IdempotentFlow, api::remittances::RemittanceCreateRequest>)
                ),
        )
        // Get remittance
        .service(
            web::resource("/remittances/{remittance_id}")
                .app_data(web::Data::new(ReqState::new("remittance_id")))
                .route(web::get().to(retrieve_remittance))
                .route(web::patch().to(update_remittance))
                .route(web::delete().to(cancel_remittance))
        )
        // Pay remittance
        .service(
            web::resource("/remittances/{remittance_id}/pay")
                .app_data(web::Data::new(ReqState::new("remittance_id")))
                .route(web::post().to(pay_remittance))
        )
        // List remittances
        .service(
            web::resource("/remittances/list")
                .app_data(web::Data::new(ReqState::new("")))
                .route(web::get().to(list_remittances))
        )
        // Sync remittances
        .service(
            web::resource("/remittances/sync")
                .app_data(web::Data::new(ReqState::new("")))
                .route(web::post().to(sync_remittances))
        )
}

/// Create a new remittance
#[instrument(skip_all)]
pub async fn create_remittance<F, R>(
    state: web::Data<AppState>,
    req: HttpRequest,
    form_payload: web::Json<R>,
    route_data: web::Data<RemittanceState<F>>,
) -> HttpResponse
where
    F: IdempotentFlow + Clone,
    R: RemittanceCreateRequest + Clone,
{
    let flow = route_data.flow.clone();
    
    // Derive auth type based on request
    let auth_flow = form_payload.auth_flow.as_ref().unwrap_or(&AuthFlow::Merchant);
    let auth = match auth_flow {
        AuthFlow::Merchant => {
            Box::new(HeaderAuth(ApiKeyAuth {
                is_connected_allowed: true,
                is_platform_allowed: true,
            })) as Box<dyn crate::routes::AppStateInfo>
        }
        AuthFlow::Client => Box::new(HeaderAuth(PublishableKeyAuth)) as Box<dyn crate::routes::AppStateInfo>,
    };

    let idempotency_key = utils::generate_id(32, "rem");
    let key = api_keys::get_api_key(&req).unwrap_or_default();
    let merchant_id = form_payload.merchant_id.clone();
    let profile_id = form_payload.profile_id.clone();

    let request_info = RemittanceRequestInfo {
        remittance_id: form_payload.remittance_id.clone().unwrap_or_default(),
        merchant_id,
        profile_id,
    };

    // Process with idempotency
    flow.handle_idempotent_request_with_lock(
        &*state,
        &req,
        form_payload.into_inner(),
        &auth,
        request_info,
        idempotency_key,
        create_remittance_inner,
        LockingAction::CreateRemittance
    )
    .await
}

async fn create_remittance_inner(
    state: &AppState,
    req: &HttpRequest,
    remittance_req: api::remittances::RemittanceCreateRequest,
    _auth_flow: &AuthFlow,
    _req_info: RemittanceRequestInfo,
) -> RouterResult<RemittanceResponse> {
    let response = remittances::create_remittance(state, remittance_req).await?;
    Ok(routes::api_response(response, HttpResponse::Created))
}

/// Retrieve a remittance
#[instrument(skip_all)]
pub async fn retrieve_remittance(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<RemittanceRetrieveRequest>,
) -> HttpResponse {
    let remittance_id = path.into_inner();
    let auth = Box::new(HeaderAuth(ApiKeyAuth {
        is_connected_allowed: true,
        is_platform_allowed: true,
    })) as Box<dyn crate::routes::AppStateInfo>;

    let response = routes::api_v2::handle_request(
        state.as_ref(),
        &req,
        auth,
        |state, auth| {
            remittances::retrieve_remittance(
                state,
                &remittance_id,
                auth.key_store,
                auth.merchant_account,
                query.force_sync,
            )
        },
    )
    .await;

    routes::api_response(response, HttpResponse::Ok)
}

/// Update a remittance
#[instrument(skip_all)]
pub async fn update_remittance(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    form_payload: web::Json<RemittanceUpdateRequest>,
    route_data: web::Data<RemittanceState<IdempotentFlow>>,
) -> HttpResponse {
    let flow = route_data.flow.clone();
    let remittance_id = path.into_inner();
    let idempotency_key = utils::generate_id(32, "remupd");
    let auth = Box::new(HeaderAuth(ApiKeyAuth {
        is_connected_allowed: true,
        is_platform_allowed: true,
    })) as Box<dyn crate::routes::AppStateInfo>;

    let request_info = RemittanceRequestInfo {
        remittance_id: remittance_id.clone(),
        merchant_id: None,
        profile_id: None,
    };

    flow.handle_idempotent_request_with_lock(
        &*state,
        &req,
        (remittance_id, form_payload.into_inner()),
        &auth,
        request_info,
        idempotency_key,
        update_remittance_inner,
        LockingAction::UpdateRemittance
    )
    .await
}

async fn update_remittance_inner(
    state: &AppState,
    _req: &HttpRequest,
    update_data: (String, RemittanceUpdateRequest),
    _auth_flow: &AuthFlow,
    _req_info: RemittanceRequestInfo,
    auth_data: crate::routes::AuthenticationData
) -> RouterResult<RemittanceResponse> {
    let (remittance_id, update_req) = update_data;
    let response = remittances::update_remittance(
        state,
        &remittance_id,
        update_req,
        auth_data.key_store,
        auth_data.merchant_account,
    ).await?;
    
    Ok(routes::api_response(response, HttpResponse::Ok))
}

/// Pay a remittance
#[instrument(skip_all)]
pub async fn pay_remittance(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    form_payload: web::Json<RemittancePayRequest>,
    route_data: web::Data<RemittanceState<IdempotentFlow>>,
) -> HttpResponse {
    let flow = route_data.flow.clone();
    let remittance_id = path.into_inner();
    let idempotency_key = utils::generate_id(32, "rempay");
    let auth = Box::new(HeaderAuth(ApiKeyAuth {
        is_connected_allowed: true,
        is_platform_allowed: true,
    })) as Box<dyn crate::routes::AppStateInfo>;

    let request_info = RemittanceRequestInfo {
        remittance_id: remittance_id.clone(),
        merchant_id: None,
        profile_id: None,
    };

    flow.handle_idempotent_request_with_lock(
        &*state,
        &req,
        (remittance_id, form_payload.into_inner()),
        &auth,
        request_info,
        idempotency_key,
        pay_remittance_inner,
        LockingAction::PayRemittance
    )
    .await
}

async fn pay_remittance_inner(
    state: &AppState,
    _req: &HttpRequest,
    pay_data: (String, RemittancePayRequest),
    _auth_flow: &AuthFlow,
    _req_info: RemittanceRequestInfo,
    auth_data: crate::routes::AuthenticationData
) -> RouterResult<RemittanceResponse> {
    let (remittance_id, pay_req) = pay_data;
    let response = remittances::pay_remittance(
        state,
        &remittance_id,
        pay_req,
        auth_data.key_store,
        auth_data.merchant_account,
    ).await?;
    
    Ok(routes::api_response(response, HttpResponse::Ok))
}

/// List remittances
#[instrument(skip_all)]
pub async fn list_remittances(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<RemittanceListRequest>,
) -> HttpResponse {
    let auth = Box::new(HeaderAuth(ApiKeyAuth {
        is_connected_allowed: true,
        is_platform_allowed: true,
    })) as Box<dyn crate::routes::AppStateInfo>;

    let response = routes::api_v2::handle_request(
        state.as_ref(),
        &req,
        auth,
        |state, auth| {
            remittances::list_remittances(
                state,
                query.into_inner(),
                auth.key_store,
                auth.merchant_account,
            )
        },
    )
    .await;

    routes::api_response(response, HttpResponse::Ok)
}

/// Cancel a remittance
#[instrument(skip_all)]
pub async fn cancel_remittance(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    route_data: web::Data<RemittanceState<IdempotentFlow>>,
) -> HttpResponse {
    let flow = route_data.flow.clone();
    let remittance_id = path.into_inner();
    let idempotency_key = utils::generate_id(32, "remcancel");
    let auth = Box::new(HeaderAuth(ApiKeyAuth {
        is_connected_allowed: true,
        is_platform_allowed: true,
    })) as Box<dyn crate::routes::AppStateInfo>;

    let request_info = RemittanceRequestInfo {
        remittance_id: remittance_id.clone(),
        merchant_id: None,
        profile_id: None,
    };

    flow.handle_idempotent_request_with_lock(
        &*state,
        &req,
        remittance_id,
        &auth,
        request_info,
        idempotency_key,
        cancel_remittance_inner,
        LockingAction::CancelRemittance
    )
    .await
}

async fn cancel_remittance_inner(
    state: &AppState,
    _req: &HttpRequest,
    remittance_id: String,
    _auth_flow: &AuthFlow,
    _req_info: RemittanceRequestInfo,
    auth_data: crate::routes::AuthenticationData
) -> RouterResult<RemittanceResponse> {
    let response = remittances::cancel_remittance(
        state,
        &remittance_id,
        auth_data.key_store,
        auth_data.merchant_account,
    ).await?;
    
    Ok(routes::api_response(response, HttpResponse::Ok))
}

/// Sync remittances
#[instrument(skip_all)]
pub async fn sync_remittances(
    state: web::Data<AppState>,
    req: HttpRequest,
    form_payload: web::Json<RemittanceSyncRequest>,
) -> HttpResponse {
    let auth = Box::new(HeaderAuth(ApiKeyAuth {
        is_connected_allowed: true,
        is_platform_allowed: true,
    })) as Box<dyn crate::routes::AppStateInfo>;

    let response = routes::api_v2::handle_request(
        state.as_ref(),
        &req,
        auth,
        |state, _auth| {
            remittances::sync_remittances(
                state,
                form_payload.into_inner(),
            )
        },
    )
    .await;

    routes::api_response(response, HttpResponse::Ok)
}