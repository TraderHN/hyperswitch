//! HTTP routes for remittances

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::remittances::{
    RemittanceCreateRequest, RemittanceListRequest, RemittanceManualUpdateRequest,
    RemittancePayRequest, RemittanceQuoteRequest, RemittanceRetrieveRequest,
    RemittanceSyncRequest, RemittanceUpdateRequest,
};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, remittances as remittances_core},
    routes::metrics,
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api as api_types,
    AppState,
};

/// Create a new remittance
#[instrument(skip_all, fields(flow = ?Flow::RemittanceCreate))]
pub async fn remittances_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<RemittanceCreateRequest>,
) -> HttpResponse {
    let flow = Flow::RemittanceCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            let (merchant_account, _, key_store, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            let profile = state
                .store
                .find_business_profile_by_profile_id(
                    &req.profile_id.unwrap_or_default(),
                    &key_store,
                )
                .await?;
            
            api_locking::check_mutation_keys_validity(
                &merchant_account.merchant_id,
                &[
                    api_locking::LockAction::NotApplicable,
                ],
            )
            .await?;
            
            remittances_core::create_remittance(
                state.clone(),
                req.into(),
                merchant_account,
                profile,
                req,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Pay (fund) a remittance
#[instrument(skip_all, fields(flow = ?Flow::RemittancePay))]
pub async fn remittances_pay(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<RemittancePayRequest>,
) -> HttpResponse {
    let flow = Flow::RemittancePay;
    let remittance_id = path.into_inner();
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            let (merchant_account, _, key_store, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            let profile = state
                .store
                .find_business_profile_by_profile_id(
                    &req.profile_id.unwrap_or_default(),
                    &key_store,
                )
                .await?;
            
            remittances_core::pay_remittance(
                state.clone(),
                req.into(),
                merchant_account,
                profile,
                remittance_id,
                req,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Retrieve a remittance
#[instrument(skip_all, fields(flow = ?Flow::RemittanceRetrieve))]
pub async fn remittances_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<RemittanceRetrieveRequest>,
) -> HttpResponse {
    let flow = Flow::RemittanceRetrieve;
    let remittance_id = path.into_inner();
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            let (merchant_account, _, key_store, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            let profile = state
                .store
                .find_business_profile_by_profile_id(
                    &req.profile_id.unwrap_or_default(),
                    &key_store,
                )
                .await?;
            
            remittances_core::retrieve_remittance(
                state.clone(),
                req.into(),
                merchant_account,
                profile,
                remittance_id,
                req,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Update a remittance
#[instrument(skip_all, fields(flow = ?Flow::RemittanceUpdate))]
pub async fn remittances_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<RemittanceUpdateRequest>,
) -> HttpResponse {
    let flow = Flow::RemittanceUpdate;
    let remittance_id = path.into_inner();
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            let (merchant_account, _, key_store, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            let profile = state
                .store
                .find_business_profile_by_profile_id(
                    &req.profile_id.unwrap_or_default(),
                    &key_store,
                )
                .await?;
            
            remittances_core::update_remittance(
                state.clone(),
                req.into(),
                merchant_account,
                profile,
                remittance_id,
                req,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Cancel a remittance
#[instrument(skip_all, fields(flow = ?Flow::RemittanceCancel))]
pub async fn remittances_cancel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RemittanceCancel;
    let remittance_id = path.into_inner();
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| async move {
            let (merchant_account, _, key_store, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            let profile = state
                .store
                .find_business_profile_by_profile_id(
                    &req.profile_id.unwrap_or_default(),
                    &key_store,
                )
                .await?;
            
            remittances_core::cancel_remittance(
                state.clone(),
                req.into(),
                merchant_account,
                profile,
                remittance_id,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// List remittances
#[instrument(skip_all, fields(flow = ?Flow::RemittanceList))]
pub async fn remittances_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<RemittanceListRequest>,
) -> HttpResponse {
    let flow = Flow::RemittanceList;
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            let (merchant_account, _, key_store, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            let profile = state
                .store
                .find_business_profile_by_profile_id(
                    &req.profile_id.unwrap_or_default(),
                    &key_store,
                )
                .await?;
            
            remittances_core::list_remittances(
                state.clone(),
                merchant_account,
                profile,
                req,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Get exchange rate quote
#[instrument(skip_all, fields(flow = ?Flow::RemittanceQuote))]
pub async fn remittances_quote(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<RemittanceQuoteRequest>,
) -> HttpResponse {
    let flow = Flow::RemittanceQuote;
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            let (merchant_account, _, key_store, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            let profile = state
                .store
                .find_business_profile_by_profile_id(
                    &req.profile_id.unwrap_or_default(),
                    &key_store,
                )
                .await?;
            
            remittances_core::get_remittance_quote(
                state.clone(),
                req.into(),
                merchant_account,
                profile,
                req,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Sync remittance status
#[instrument(skip_all, fields(flow = ?Flow::RemittanceSync))]
pub async fn remittances_sync(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RemittanceSync;
    let remittance_id = path.into_inner();
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| async move {
            let (merchant_account, _, key_store, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            let profile = state
                .store
                .find_business_profile_by_profile_id(
                    &req.profile_id.unwrap_or_default(),
                    &key_store,
                )
                .await?;
            
            remittances_core::sync_remittance(
                state.clone(),
                req.into(),
                merchant_account,
                profile,
                remittance_id,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Sync multiple remittances
#[instrument(skip_all, fields(flow = ?Flow::RemittanceSyncBatch))]
pub async fn remittances_sync_batch(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<RemittanceSyncRequest>,
) -> HttpResponse {
    let flow = Flow::RemittanceSyncBatch;
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            let (merchant_account, _, _, _) = auth
                .get_merchant_account_key_store_profile_id_from_header(&req.headers())
                .await?;
            
            remittances_core::sync_remittances_batch(
                state.clone(),
                req.into(),
                merchant_account,
                req,
            )
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::RemittanceWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Manual update remittance (admin only)
#[instrument(skip_all, fields(flow = ?Flow::RemittanceManualUpdate))]
pub async fn remittances_manual_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<RemittanceManualUpdateRequest>,
) -> HttpResponse {
    let flow = Flow::RemittanceManualUpdate;
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| async move {
            auth.check_admin_or_operator_permissions()?;
            
            remittances_core::manual_update_remittance(
                state.clone(),
                req,
            )
            .await
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuth {
                permission: Permission::OperationsManage,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Webhook handler for remittances
#[instrument(skip_all, fields(flow = ?Flow::RemittanceWebhook))]
pub async fn remittances_webhook(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Bytes,
) -> HttpResponse {
    let flow = Flow::RemittanceWebhook;
    let (merchant_id, connector_name) = path.into_inner();
    
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        body,
        |state, _, body, _| async move {
            let body_str = std::str::from_utf8(&body)
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "body".to_string(),
                    expected_format: "UTF-8 string".to_string(),
                })?;
            
            let body_json: serde_json::Value = serde_json::from_str(body_str)
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "body".to_string(),
                    expected_format: "valid JSON".to_string(),
                })?;
            
            remittances_core::handle_remittance_webhook(
                state.clone(),
                req.into(),
                merchant_id,
                connector_name,
                body_json,
            )
            .await
        },
        auth::auth_type(
            &auth::WebhookAuth {
                connector: &connector_name,
                merchant_id: &merchant_id,
            },
            &auth::NoAuth,
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}