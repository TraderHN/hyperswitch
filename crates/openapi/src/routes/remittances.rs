/// Remittances - Create
#[utoipa::path(
    post,
    path = "/remittances/create",
    request_body=RemittanceCreateRequest,
    responses(
        (status = 200, description = "Remittance created", body = RemittanceResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Remittances",
    operation_id = "Create a Remittance",
    security(("api_key" = []))
)]
pub async fn remittances_create() {}

/// Remittances - Retrieve
#[utoipa::path(
    get,
    path = "/remittances/{remittance_id}",
    params(
        ("remittance_id" = String, Path, description = "The identifier for remittance"),
        ("force_sync" = Option<bool>, Query, description = "Sync with the connector to get the remittance details (defaults to false)"),
        ("client_secret" = Option<String>, Query, description = "Client secret for authenticated access")
    ),
    responses(
        (status = 200, description = "Remittance retrieved", body = RemittanceResponse),
        (status = 404, description = "Remittance does not exist in our records")
    ),
    tag = "Remittances",
    operation_id = "Retrieve a Remittance",
    security(("api_key" = []))
)]
pub async fn remittances_retrieve() {}

/// Remittances - Update
#[utoipa::path(
    post,
    path = "/remittances/{remittance_id}",
    params(
        ("remittance_id" = String, Path, description = "The identifier for remittance")
    ),
    request_body=RemittanceUpdateRequest,
    responses(
        (status = 200, description = "Remittance updated", body = RemittanceResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Remittances",
    operation_id = "Update a Remittance",
    security(("api_key" = []))
)]
pub async fn remittances_update() {}

/// Remittances - Pay
#[utoipa::path(
    post,
    path = "/remittances/{remittance_id}/pay",
    params(
        ("remittance_id" = String, Path, description = "The identifier for remittance")
    ),
    request_body=RemittancePayRequest,
    responses(
        (status = 200, description = "Remittance payment processed", body = RemittanceResponse),
        (status = 400, description = "Missing Mandatory fields"),
        (status = 402, description = "Payment failed")
    ),
    tag = "Remittances",
    operation_id = "Pay for a Remittance",
    security(("api_key" = []))
)]
pub async fn remittances_pay() {}

/// Remittances - Cancel
#[utoipa::path(
    delete,
    path = "/remittances/{remittance_id}",
    params(
        ("remittance_id" = String, Path, description = "The identifier for remittance")
    ),
    responses(
        (status = 200, description = "Remittance cancelled", body = RemittanceResponse),
        (status = 400, description = "Remittance cannot be cancelled"),
        (status = 404, description = "Remittance not found")
    ),
    tag = "Remittances",
    operation_id = "Cancel a Remittance",
    security(("api_key" = []))
)]
pub async fn remittances_cancel() {}

/// Remittances - List
#[utoipa::path(
    get,
    path = "/remittances/list",
    params(
        ("status" = Option<Vec<String>>, Query, description = "Filter by remittance status"),
        ("connector" = Option<String>, Query, description = "Filter by connector"),
        ("source_currency" = Option<String>, Query, description = "Filter by source currency"),
        ("destination_currency" = Option<String>, Query, description = "Filter by destination currency"),
        ("limit" = Option<u32>, Query, description = "Limit on the number of objects to return (max 100, default 10)"),
        ("offset" = Option<u32>, Query, description = "Number of objects to skip for pagination"),
        ("time_range" = Option<String>, Query, description = "The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).")
    ),
    responses(
        (status = 200, description = "Remittances listed", body = RemittanceListResponse),
        (status = 404, description = "Remittances not found")
    ),
    tag = "Remittances",
    operation_id = "List remittances using query parameters",
    security(("api_key" = []))
)]
pub async fn remittances_list() {}

/// Remittances - List using filters
#[utoipa::path(
    post,
    path = "/remittances/list",
    request_body=RemittanceListRequest,
    responses(
        (status = 200, description = "Remittances filtered", body = RemittanceListResponse),
        (status = 404, description = "Remittances not found")
    ),
    tag = "Remittances",
    operation_id = "Filter remittances using specific constraints",
    security(("api_key" = []))
)]
pub async fn remittances_list_by_filter() {}

/// Remittances - Get Quote
#[utoipa::path(
    post,
    path = "/remittances/quote",
    request_body=RemittanceQuoteRequest,
    responses(
        (status = 200, description = "Quote retrieved", body = RemittanceQuoteResponse),
        (status = 400, description = "Invalid request parameters")
    ),
    tag = "Remittances",
    operation_id = "Get exchange rate quote for remittance",
    security(("api_key" = []))
)]
pub async fn remittances_quote() {}

/// Remittances - Sync Status
#[utoipa::path(
    post,
    path = "/remittances/sync",
    request_body=RemittanceSyncRequest,
    responses(
        (status = 200, description = "Remittances synchronized", body = RemittanceSyncResponse),
        (status = 400, description = "Invalid request parameters")
    ),
    tag = "Remittances",
    operation_id = "Sync remittance statuses with connector",
    security(("api_key" = []))
)]
pub async fn remittances_sync() {}

/// Remittances - Manual Update (Admin)
#[utoipa::path(
    post,
    path = "/remittances/{remittance_id}/manual_update",
    params(
        ("remittance_id" = String, Path, description = "The identifier for remittance")
    ),
    request_body=RemittanceManualUpdateRequest,
    responses(
        (status = 200, description = "Remittance manually updated", body = RemittanceResponse),
        (status = 400, description = "Invalid request parameters"),
        (status = 403, description = "Unauthorized - admin access required")
    ),
    tag = "Remittances",
    operation_id = "Manually update remittance status (Admin only)",
    security(("admin_api_key" = []))
)]
pub async fn remittances_manual_update() {}

/// Remittances - Sync by Merchant
#[utoipa::path(
    post,
    path = "/remittances/sync/merchant",
    request_body=RemittanceSyncRequest,
    responses(
        (status = 200, description = "Merchant remittances synchronized", body = RemittanceSyncResponse),
        (status = 400, description = "Invalid request parameters")
    ),
    tag = "Remittances",
    operation_id = "Sync all remittances for a merchant",
    security(("api_key" = []))
)]
pub async fn remittances_sync_merchant() {}

/// Remittances - Sync by Profile
#[utoipa::path(
    post,
    path = "/remittances/sync/profile",
    request_body=RemittanceSyncRequest,
    responses(
        (status = 200, description = "Profile remittances synchronized", body = RemittanceSyncResponse),
        (status = 400, description = "Invalid request parameters")
    ),
    tag = "Remittances",
    operation_id = "Sync all remittances for a profile",
    security(("api_key" = []))
)]
pub async fn remittances_sync_profile() {}

/// Remittances - List available filters
#[utoipa::path(
    post,
    path = "/remittances/filter",
    request_body=TimeRange,
    responses(
        (status = 200, description = "Filters listed", body = RemittanceListFilters)
    ),
    tag = "Remittances",
    operation_id = "List available remittance filters",
    security(("api_key" = []))
)]
pub async fn remittances_list_filters() {}