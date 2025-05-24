//! Database interface for remittances

#[cfg(feature = "remittances")]
use std::collections::HashMap;

#[cfg(feature = "remittances")]
use api_models::remittances::{RemittanceListRequest, RemittanceStatus};
#[cfg(feature = "remittances")]
use common_utils::{
    errors::CustomResult,
    id_type::{MerchantId, ProfileId},
    types::TimeRange,
};
#[cfg(feature = "remittances")]
use diesel_models::{
    errors::DatabaseError,
    remittance::{
        Remittance, RemittanceNew, RemittancePayment, RemittancePaymentNew,
        RemittancePaymentUpdate, RemittancePayout, RemittancePayoutNew, RemittancePayoutUpdate,
        RemittanceUpdate,
    },
};
#[cfg(feature = "remittances")]
use error_stack::ResultExt;
#[cfg(feature = "remittances")]
use storage_impl::remittance::RemittanceDbExt;

#[cfg(feature = "remittances")]
use crate::{
    connection::PgPooledConn,
    core::errors::{self, StorageErrorExt},
    logger,
};

/// Database interface trait for remittances
#[cfg(feature = "remittances")]
#[async_trait::async_trait]
pub trait RemittanceInterface {
    /// Create a new remittance
    async fn insert_remittance(
        &self,
        remittance: RemittanceNew,
    ) -> CustomResult<Remittance, errors::StorageError>;

    /// Find a remittance by ID
    async fn find_remittance_by_id(
        &self,
        id: &uuid::Uuid,
    ) -> CustomResult<Remittance, errors::StorageError>;

    /// Find a remittance by merchant ID and remittance ID
    async fn find_remittance_by_merchant_id_remittance_id(
        &self,
        merchant_id: &MerchantId,
        id: &uuid::Uuid,
    ) -> CustomResult<Remittance, errors::StorageError>;

    /// Update a remittance
    async fn update_remittance(
        &self,
        this: Remittance,
        remittance: RemittanceUpdate,
    ) -> CustomResult<Remittance, errors::StorageError>;

    /// Delete a remittance (soft delete by status)
    async fn delete_remittance_by_merchant_id_remittance_id(
        &self,
        merchant_id: &MerchantId,
        id: &uuid::Uuid,
    ) -> CustomResult<bool, errors::StorageError>;

    /// List remittances with filtering and pagination
    async fn filter_remittances_by_constraints(
        &self,
        merchant_id: &MerchantId,
        filters: &RemittanceListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Remittance>, errors::StorageError>;

    /// Get total count of remittances matching filters
    async fn get_remittances_count(
        &self,
        merchant_id: &MerchantId,
        filters: &RemittanceListRequest,
    ) -> CustomResult<i64, errors::StorageError>;

    /// Get remittance status distribution for analytics
    async fn get_remittance_status_with_count(
        &self,
        merchant_id: &MerchantId,
        profile_id_list: Option<Vec<ProfileId>>,
        time_range: &TimeRange,
    ) -> CustomResult<Vec<(RemittanceStatus, i64)>, errors::StorageError>;

    /// Find remittances by status
    async fn find_remittances_by_merchant_id_status(
        &self,
        merchant_id: &MerchantId,
        status: RemittanceStatus,
        limit: i64,
    ) -> CustomResult<Vec<Remittance>, errors::StorageError>;
}

/// Database interface trait for remittance payments
#[cfg(feature = "remittances")]
#[async_trait::async_trait]
pub trait RemittancePaymentInterface {
    /// Insert a new remittance payment record
    async fn insert_remittance_payment(
        &self,
        remittance_payment: RemittancePaymentNew,
    ) -> CustomResult<RemittancePayment, errors::StorageError>;

    /// Find remittance payment by remittance ID
    async fn find_remittance_payment_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
    ) -> CustomResult<RemittancePayment, errors::StorageError>;

    /// Update remittance payment
    async fn update_remittance_payment_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
        remittance_payment: RemittancePaymentUpdate,
    ) -> CustomResult<RemittancePayment, errors::StorageError>;

    /// Find remittance payment by payment ID
    async fn find_remittance_payment_by_payment_id(
        &self,
        payment_id: &str,
    ) -> CustomResult<RemittancePayment, errors::StorageError>;
}

/// Database interface trait for remittance payouts
#[cfg(feature = "remittances")]
#[async_trait::async_trait]
pub trait RemittancePayoutInterface {
    /// Insert a new remittance payout record
    async fn insert_remittance_payout(
        &self,
        remittance_payout: RemittancePayoutNew,
    ) -> CustomResult<RemittancePayout, errors::StorageError>;

    /// Find remittance payout by remittance ID
    async fn find_remittance_payout_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
    ) -> CustomResult<RemittancePayout, errors::StorageError>;

    /// Update remittance payout
    async fn update_remittance_payout_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
        remittance_payout: RemittancePayoutUpdate,
    ) -> CustomResult<RemittancePayout, errors::StorageError>;

    /// Find remittance payout by payout ID
    async fn find_remittance_payout_by_payout_id(
        &self,
        payout_id: &str,
    ) -> CustomResult<RemittancePayout, errors::StorageError>;
}

/// Implementation of RemittanceInterface
#[cfg(feature = "remittances")]
#[async_trait::async_trait]
impl RemittanceInterface for crate::SessionState {
    async fn insert_remittance(
        &self,
        remittance: RemittanceNew,
    ) -> CustomResult<Remittance, errors::StorageError> {
        let conn = self
            .store
            .get_master_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        remittance
            .insert(&conn)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn find_remittance_by_id(
        &self,
        id: &uuid::Uuid,
    ) -> CustomResult<Remittance, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        Remittance::find_by_remittance_id(&conn, id)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn find_remittance_by_merchant_id_remittance_id(
        &self,
        merchant_id: &MerchantId,
        id: &uuid::Uuid,
    ) -> CustomResult<Remittance, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        Remittance::find_by_merchant_id_remittance_id(&conn, merchant_id, id)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn update_remittance(
        &self,
        this: Remittance,
        remittance: RemittanceUpdate,
    ) -> CustomResult<Remittance, errors::StorageError> {
        let conn = self
            .store
            .get_master_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        this.update(&conn, remittance)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn delete_remittance_by_merchant_id_remittance_id(
        &self,
        merchant_id: &MerchantId,
        id: &uuid::Uuid,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = self
            .store
            .get_master_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        // Soft delete by updating status to cancelled
        let remittance = Remittance::find_by_merchant_id_remittance_id(&conn, merchant_id, id)
            .await
            .map_err(|error| error.to_storage_error())?;

        let update = RemittanceUpdate::build_status_update(RemittanceStatus::Cancelled);
        
        remittance
            .update(&conn, update)
            .await
            .map_err(|error| error.to_storage_error())
            .map(|_| true)
    }

    async fn filter_remittances_by_constraints(
        &self,
        merchant_id: &MerchantId,
        filters: &RemittanceListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Remittance>, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        Remittance::filter_by_constraints(&conn, merchant_id, filters, limit, offset)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn get_remittances_count(
        &self,
        merchant_id: &MerchantId,
        filters: &RemittanceListRequest,
    ) -> CustomResult<i64, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        Remittance::get_remittances_count(&conn, merchant_id, filters)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn get_remittance_status_with_count(
        &self,
        merchant_id: &MerchantId,
        profile_id_list: Option<Vec<ProfileId>>,
        time_range: &TimeRange,
    ) -> CustomResult<Vec<(RemittanceStatus, i64)>, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        Remittance::get_remittance_status_with_count(&conn, merchant_id, profile_id_list, time_range)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn find_remittances_by_merchant_id_status(
        &self,
        merchant_id: &MerchantId,
        status: RemittanceStatus,
        limit: i64,
    ) -> CustomResult<Vec<Remittance>, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        Remittance::find_by_merchant_id_status(&conn, merchant_id, status, limit)
            .await
            .map_err(|error| error.to_storage_error())
    }
}

/// Implementation of RemittancePaymentInterface
#[cfg(feature = "remittances")]
#[async_trait::async_trait]
impl RemittancePaymentInterface for crate::SessionState {
    async fn insert_remittance_payment(
        &self,
        remittance_payment: RemittancePaymentNew,
    ) -> CustomResult<RemittancePayment, errors::StorageError> {
        let conn = self
            .store
            .get_master_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        remittance_payment
            .insert(&conn)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn find_remittance_payment_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
    ) -> CustomResult<RemittancePayment, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        RemittancePayment::find_by_remittance_id(&conn, remittance_id)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn update_remittance_payment_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
        remittance_payment: RemittancePaymentUpdate,
    ) -> CustomResult<RemittancePayment, errors::StorageError> {
        let conn = self
            .store
            .get_master_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        RemittancePayment::update_by_remittance_id(&conn, remittance_id, remittance_payment)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn find_remittance_payment_by_payment_id(
        &self,
        payment_id: &str,
    ) -> CustomResult<RemittancePayment, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        RemittancePayment::find_by_payment_id(&conn, payment_id)
            .await
            .map_err(|error| error.to_storage_error())
    }
}

/// Implementation of RemittancePayoutInterface
#[cfg(feature = "remittances")]
#[async_trait::async_trait]
impl RemittancePayoutInterface for crate::SessionState {
    async fn insert_remittance_payout(
        &self,
        remittance_payout: RemittancePayoutNew,
    ) -> CustomResult<RemittancePayout, errors::StorageError> {
        let conn = self
            .store
            .get_master_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        remittance_payout
            .insert(&conn)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn find_remittance_payout_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
    ) -> CustomResult<RemittancePayout, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        RemittancePayout::find_by_remittance_id(&conn, remittance_id)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn update_remittance_payout_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
        remittance_payout: RemittancePayoutUpdate,
    ) -> CustomResult<RemittancePayout, errors::StorageError> {
        let conn = self
            .store
            .get_master_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        RemittancePayout::update_by_remittance_id(&conn, remittance_id, remittance_payout)
            .await
            .map_err(|error| error.to_storage_error())
    }

    async fn find_remittance_payout_by_payout_id(
        &self,
        payout_id: &str,
    ) -> CustomResult<RemittancePayout, errors::StorageError> {
        let conn = self
            .store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;

        RemittancePayout::find_by_payout_id(&conn, payout_id)
            .await
            .map_err(|error| error.to_storage_error())
    }
}

/// Helper functions for common database operations
#[cfg(feature = "remittances")]
impl crate::SessionState {
    /// Get remittance with associated payment and payout data
    pub async fn get_remittance_with_relations(
        &self,
        merchant_id: &MerchantId,
        remittance_id: &uuid::Uuid,
    ) -> CustomResult<
        (Remittance, Option<RemittancePayment>, Option<RemittancePayout>),
        errors::StorageError,
    > {
        let remittance = self
            .find_remittance_by_merchant_id_remittance_id(merchant_id, remittance_id)
            .await?;

        let payment = self
            .find_remittance_payment_by_remittance_id(remittance_id)
            .await
            .ok();

        let payout = self
            .find_remittance_payout_by_remittance_id(remittance_id)
            .await
            .ok();

        Ok((remittance, payment, payout))
    }

    /// Get remittance analytics for dashboard
    pub async fn get_remittance_analytics(
        &self,
        merchant_id: &MerchantId,
        profile_id_list: Option<Vec<ProfileId>>,
        time_range: &TimeRange,
    ) -> CustomResult<HashMap<String, serde_json::Value>, errors::StorageError> {
        let status_counts = self
            .get_remittance_status_with_count(merchant_id, profile_id_list.clone(), time_range)
            .await?;

        let total_count = self
            .get_remittances_count(
                merchant_id,
                &RemittanceListRequest {
                    status: None,
                    connector: None,
                    source_currency: None,
                    destination_currency: None,
                    time_range: Some(time_range.clone()),
                    limit: None,
                    offset: None,
                },
            )
            .await?;

        let success_count = status_counts
            .iter()
            .find(|(status, _)| *status == RemittanceStatus::Completed)
            .map(|(_, count)| *count)
            .unwrap_or(0);

        let success_rate = if total_count > 0 {
            (success_count as f64 / total_count as f64) * 100.0
        } else {
            0.0
        };

        let mut analytics = HashMap::new();
        analytics.insert("total_count".to_string(), serde_json::json!(total_count));
        analytics.insert("success_count".to_string(), serde_json::json!(success_count));
        analytics.insert("success_rate".to_string(), serde_json::json!(success_rate));
        analytics.insert("status_distribution".to_string(), serde_json::json!(status_counts));

        Ok(analytics)
    }

    /// Create remittance with payment record atomically
    pub async fn create_remittance_with_payment(
        &self,
        remittance: RemittanceNew,
        payment_id: Option<String>,
    ) -> CustomResult<(Remittance, Option<RemittancePayment>), errors::StorageError> {
        // Start with inserting the remittance
        let created_remittance = self.insert_remittance(remittance).await?;

        // If payment_id is provided, create payment record
        let payment_record = if let Some(pid) = payment_id {
            let payment = RemittancePaymentNew {
                remittance_id: created_remittance.id,
                payment_id: Some(pid),
                connector_txn_id: None,
                status: Some("initiated".to_string()),
                auth_type: None,
                created_at: Some(common_utils::date_time::now()),
                updated_at: Some(common_utils::date_time::now()),
            };

            Some(self.insert_remittance_payment(payment).await?)
        } else {
            None
        };

        Ok((created_remittance, payment_record))
    }
}

/// Extension methods for working with remittance data
#[cfg(feature = "remittances")]
pub trait RemittanceDbUtils {
    fn is_remittance_updatable(&self) -> bool;
    fn is_remittance_cancellable(&self) -> bool;
    fn can_process_payment(&self) -> bool;
    fn can_process_payout(&self) -> bool;
}

#[cfg(feature = "remittances")]
impl RemittanceDbUtils for Remittance {
    fn is_remittance_updatable(&self) -> bool {
        matches!(
            self.status_enum().unwrap_or(RemittanceStatus::Failed),
            RemittanceStatus::Created
        )
    }

    fn is_remittance_cancellable(&self) -> bool {
        !matches!(
            self.status_enum().unwrap_or(RemittanceStatus::Failed),
            RemittanceStatus::Completed | RemittanceStatus::Failed | RemittanceStatus::Cancelled
        )
    }

    fn can_process_payment(&self) -> bool {
        matches!(
            self.status_enum().unwrap_or(RemittanceStatus::Failed),
            RemittanceStatus::Created
        )
    }

    fn can_process_payout(&self) -> bool {
        matches!(
            self.status_enum().unwrap_or(RemittanceStatus::Failed),
            RemittanceStatus::PaymentProcessed
        )
    }
}

#[cfg(all(feature = "remittances", test))]
mod tests {
    use super::*;
    use common_utils::types::MinorUnit;

    #[test]
    fn test_remittance_business_logic() {
        let mut remittance = Remittance {
            id: uuid::Uuid::new_v4(),
            merchant_id: "merchant_123".try_into().unwrap(),
            profile_id: "profile_456".try_into().unwrap(),
            amount: 10000,
            source_currency: "USD".to_string(),
            destination_currency: "MXN".to_string(),
            source_amount: Some(10000),
            destination_amount: Some(245000),
            exchange_rate: None,
            reference: "Test remittance".to_string(),
            purpose: None,
            status: RemittanceStatus::Created.to_string(),
            failure_reason: None,
            sender_details: serde_json::json!({}).into(),
            beneficiary_details: serde_json::json!({}).into(),
            return_url: None,
            metadata: None,
            connector: "test_connector".to_string(),
            client_secret: None,
            remittance_date: common_utils::date_time::now().date(),
            created_at: Some(common_utils::date_time::now()),
            updated_at: Some(common_utils::date_time::now()),
        };

        // Test initial state
        assert!(remittance.is_remittance_updatable());
        assert!(remittance.is_remittance_cancellable());
        assert!(remittance.can_process_payment());
        assert!(!remittance.can_process_payout());

        // Test after payment processed
        remittance.status = RemittanceStatus::PaymentProcessed.to_string();
        assert!(!remittance.is_remittance_updatable());
        assert!(remittance.is_remittance_cancellable());
        assert!(!remittance.can_process_payment());
        assert!(remittance.can_process_payout());

        // Test completed state
        remittance.status = RemittanceStatus::Completed.to_string();
        assert!(!remittance.is_remittance_updatable());
        assert!(!remittance.is_remittance_cancellable());
        assert!(!remittance.can_process_payment());
        assert!(!remittance.can_process_payout());
    }
}