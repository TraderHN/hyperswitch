//! Implementación de almacenamiento para remesas
//!
//! Este módulo proporciona las interfaces de almacenamiento para:
//! - Remittances (creación, consulta, actualización)
//! - RemittancePayments (registro y actualización de pagos de remesas)
//! - RemittancePayouts (registro y actualización de liquidaciones)

use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    id_type::{MerchantId, ProfileId},
    types::keymanager::KeyManagerState,
};
use common_enums::enums::{MerchantStorageScheme, RemittanceStatus};
use router_env::{instrument, tracing};

use crate::{
    errors,
    DatabaseStore, RouterStore,
};

use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;
use hyperswitch_domain_models::remittances::{
    Remittance as DomainRemittance, RemittanceInterface,
    RemittancePayment as DomainRemittancePayment, RemittancePaymentInterface,
    RemittancePayout as DomainRemittancePayout, RemittancePayoutInterface,
};

#[cfg(feature = "remittances")]
use diesel_models::remittances::{
    RemittancePaymentUpdate, RemittancePayoutUpdate, RemittanceUpdateInternal,
};

// ===== RouterStore Implementation =====

#[cfg(feature = "remittances")]
#[async_trait]
impl<T: DatabaseStore> RemittanceInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn find_remittance_by_id(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance_id: &uuid::Uuid,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainRemittance, errors::StorageError> {
        // TODO: Implement when diesel models are ready
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn find_remittance_by_merchant_id_reference(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _merchant_id: &MerchantId,
        _reference: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainRemittance, errors::StorageError> {
        // TODO: Implement when diesel models are ready
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn insert_remittance(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance: DomainRemittance,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainRemittance, errors::StorageError> {
        // TODO: Implement when diesel models are ready
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn update_remittance(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance: DomainRemittance,
        _remittance_update: RemittanceUpdateInternal,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainRemittance, errors::StorageError> {
        // TODO: Implement when diesel models are ready
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn find_remittances_by_merchant_id_profile_id(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _merchant_id: &MerchantId,
        _profile_id: &ProfileId,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainRemittance>, errors::StorageError> {
        // TODO: Implement when diesel models are ready
        Ok(vec![])
    }

    #[instrument(skip_all)]
    async fn find_remittances_by_merchant_id_status(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _merchant_id: &MerchantId,
        _status: RemittanceStatus,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainRemittance>, errors::StorageError> {
        // TODO: Implement when diesel models are ready
        Ok(vec![])
    }
}

#[cfg(feature = "remittances")]
#[async_trait]
impl<T: DatabaseStore> RemittancePaymentInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn find_remittance_payment_by_remittance_id(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance_id: &uuid::Uuid,
    ) -> CustomResult<DomainRemittancePayment, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn insert_remittance_payment(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance_payment: DomainRemittancePayment,
    ) -> CustomResult<DomainRemittancePayment, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn update_remittance_payment(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance_payment: DomainRemittancePayment,
        _update: RemittancePaymentUpdate,
    ) -> CustomResult<DomainRemittancePayment, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[cfg(feature = "remittances")]
#[async_trait]
impl<T: DatabaseStore> RemittancePayoutInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn find_remittance_payout_by_remittance_id(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance_id: &uuid::Uuid,
    ) -> CustomResult<DomainRemittancePayout, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn insert_remittance_payout(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance_payout: DomainRemittancePayout,
    ) -> CustomResult<DomainRemittancePayout, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn update_remittance_payout(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _remittance_payout: DomainRemittancePayout,
        _update: RemittancePayoutUpdate,
    ) -> CustomResult<DomainRemittancePayout, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

// ===== Implementations for when remittances feature is disabled =====

#[cfg(not(feature = "remittances"))]
impl<T: DatabaseStore> RemittanceInterface for RouterStore<T> {}

#[cfg(not(feature = "remittances"))]
impl<T: DatabaseStore> RemittancePaymentInterface for RouterStore<T> {}

#[cfg(not(feature = "remittances"))]
impl<T: DatabaseStore> RemittancePayoutInterface for RouterStore<T> {}