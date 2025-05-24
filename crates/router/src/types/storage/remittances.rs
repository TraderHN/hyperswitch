
#![cfg(feature = "remittances")]

#[cfg(feature = "remittances")]
use api_models::remittances::{RemittanceListRequest, RemittanceStatus};
use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl};

#[cfg(feature = "remittances")]
pub use diesel_models::remittance::{
    Remittance, RemittanceCoreWorkflow, RemittanceNew, RemittancePayment, RemittancePaymentNew,
    RemittancePaymentUpdate, RemittancePayout, RemittancePayoutNew, RemittancePayoutUpdate,
    RemittanceUpdate, RemittanceUpdateInternal,
};

// si no estás compilando con --features v2
#[cfg(all(feature = "remittances", not(feature = "v2")))]
use diesel_models::schema::remittances::dsl;

// si estás compilando con --features v2
#[cfg(all(feature = "remittances", feature = "v2"))]
use diesel_models::schema_v2::remittances::dsl;

use diesel_models::{
    enums::Currency,
    errors,
    query::generics::db_metrics,
};
use error_stack::ResultExt;

use crate::{connection::PgPooledConn, logger};

#[cfg(feature = "remittances")]
#[async_trait::async_trait]
pub trait RemittanceDbExt: Sized {
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "remittances_v2")))]
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        remittance_list_details: &RemittanceListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;

    #[cfg(all(feature = "v2", feature = "remittances_v2"))]
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        remittance_list_details: RemittanceListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "remittances_v2")))]
    async fn get_remittances_count(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        remittance_list_details: &RemittanceListRequest,
    ) -> CustomResult<i64, errors::DatabaseError>;

    #[cfg(all(feature = "v2", feature = "remittances_v2"))]
    async fn get_remittances_count(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        remittance_list_details: RemittanceListRequest,
    ) -> CustomResult<i64, errors::DatabaseError>;

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "remittances_v2")))]
    async fn get_remittance_status_with_count(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(RemittanceStatus, i64)>, errors::DatabaseError>;
}

#[cfg(feature = "remittances")]
#[async_trait::async_trait]
impl RemittanceDbExt for Remittance {
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "remittances_v2")))]
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        remittance_list_details: &RemittanceListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::updated_at.desc())
            .into_boxed();

        if let Some(status_list) = &remittance_list_details.status {
            filter = filter.filter(dsl::status.eq_any(
                status_list
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            ));
        }

        if let Some(connector) = &remittance_list_details.connector {
            filter = filter.filter(dsl::connector.eq(connector.to_owned()));
        }

        if let Some(source_currency) = &remittance_list_details.source_currency {
            filter = filter.filter(dsl::source_currency.eq(source_currency.to_string()));
        }

        if let Some(destination_currency) = &remittance_list_details.destination_currency {
            filter = filter.filter(dsl::destination_currency.eq(destination_currency.to_string()));
        }

        if let Some(time_range) = &remittance_list_details.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        filter = filter.limit(limit).offset(offset);

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_results_async(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering remittances by constraints")
    }

    #[cfg(all(feature = "v2", feature = "remittances_v2"))]
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        remittance_list_details: RemittanceListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::updated_at.desc())
            .into_boxed();

        if let Some(status_list) = remittance_list_details.status {
            filter = filter.filter(dsl::status.eq_any(
                status_list
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            ));
        }

        if let Some(connector) = remittance_list_details.connector {
            filter = filter.filter(dsl::connector.eq(connector));
        }

        if let Some(source_currency) = remittance_list_details.source_currency {
            filter = filter.filter(dsl::source_currency.eq(source_currency.to_string()));
        }

        if let Some(destination_currency) = remittance_list_details.destination_currency {
            filter = filter.filter(dsl::destination_currency.eq(destination_currency.to_string()));
        }

        if let Some(time_range) = remittance_list_details.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        filter = filter.limit(limit).offset(offset);

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_results_async(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering remittances by constraints")
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "remittances_v2")))]
    async fn get_remittances_count(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        remittance_list_details: &RemittanceListRequest,
    ) -> CustomResult<i64, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .count()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .into_boxed();

        if let Some(status_list) = &remittance_list_details.status {
            filter = filter.filter(dsl::status.eq_any(
                status_list
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            ));
        }

        if let Some(connector) = &remittance_list_details.connector {
            filter = filter.filter(dsl::connector.eq(connector.to_owned()));
        }

        if let Some(source_currency) = &remittance_list_details.source_currency {
            filter = filter.filter(dsl::source_currency.eq(source_currency.to_string()));
        }

        if let Some(destination_currency) = &remittance_list_details.destination_currency {
            filter = filter.filter(dsl::destination_currency.eq(destination_currency.to_string()));
        }

        if let Some(time_range) = &remittance_list_details.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        filter
            .get_result_async::<i64>(conn)
            .await
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable_lazy(|| "Error getting remittances count")
    }

    #[cfg(all(feature = "v2", feature = "remittances_v2"))]
    async fn get_remittances_count(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        remittance_list_details: RemittanceListRequest,
    ) -> CustomResult<i64, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .count()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .into_boxed();

        if let Some(status_list) = remittance_list_details.status {
            filter = filter.filter(dsl::status.eq_any(
                status_list
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            ));
        }

        if let Some(connector) = remittance_list_details.connector {
            filter = filter.filter(dsl::connector.eq(connector));
        }

        if let Some(source_currency) = remittance_list_details.source_currency {
            filter = filter.filter(dsl::source_currency.eq(source_currency.to_string()));
        }

        if let Some(destination_currency) = remittance_list_details.destination_currency {
            filter = filter.filter(dsl::destination_currency.eq(destination_currency.to_string()));
        }

        if let Some(time_range) = remittance_list_details.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        filter
            .get_result_async::<i64>(conn)
            .await
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable_lazy(|| "Error getting remittances count")
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "remittances_v2")))]
    async fn get_remittance_status_with_count(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(RemittanceStatus, i64)>, errors::DatabaseError> {
        let mut query = <Self as HasTable>::table()
            .group_by(dsl::status)
            .select((dsl::status, diesel::dsl::count_star()))
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .into_boxed();

        if let Some(profile_id) = profile_id_list {
            query = query.filter(dsl::profile_id.eq_any(profile_id));
        }

        query = query.filter(dsl::created_at.ge(time_range.start_time));

        query = match time_range.end_time {
            Some(ending_at) => query.filter(dsl::created_at.le(ending_at)),
            None => query,
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        let results: Vec<(String, i64)> =
            db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
                query.get_results_async::<(String, i64)>(conn),
                db_metrics::DatabaseOperation::Count,
            )
            .await
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable_lazy(|| "Error filtering status count of remittances")?;

        let converted_results: Vec<(RemittanceStatus, i64)> = results
            .into_iter()
            .filter_map(|(status_str, count)| {
                status_str
                    .parse::<RemittanceStatus>()
                    .ok()
                    .map(|status| (status, count))
            })
            .collect();

        Ok(converted_results)
    }
}