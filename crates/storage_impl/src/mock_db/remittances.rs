use common_utils::errors::CustomResult;
use diesel_models::enums as storage_enums;
use hyperswitch_domain_models::remittances::{
    Remittance, RemittanceInterface, RemittancePayment, RemittancePaymentInterface,
    RemittancePayout, RemittancePayoutInterface,
};

use crate::{errors::StorageError, MockDb};

#[async_trait::async_trait]
impl RemittanceInterface for MockDb {
    type Error = StorageError;

    async fn find_remittance_by_id(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance_id: &uuid::Uuid,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Remittance, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittanceInterface::find_remittance_by_id not implemented for MockDb".to_string()))?
    }

    async fn find_remittance_by_merchant_id_reference(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _reference: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Remittance, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittanceInterface::find_remittance_by_merchant_id_reference not implemented for MockDb".to_string()))?
    }

    async fn insert_remittance(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance: Remittance,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Remittance, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittanceInterface::insert_remittance not implemented for MockDb".to_string()))?
    }

    async fn update_remittance(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance: Remittance,
        _remittance_update: diesel_models::remittances::RemittanceUpdateInternal,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Remittance, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittanceInterface::update_remittance not implemented for MockDb".to_string()))?
    }

    async fn find_remittances_by_merchant_id_profile_id(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _profile_id: &common_utils::id_type::ProfileId,
        _limit: Option<i64>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<Remittance>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittanceInterface::find_remittances_by_merchant_id_profile_id not implemented for MockDb".to_string()))?
    }

    async fn find_remittances_by_merchant_id_status(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _merchant_id: &common_utils::id_type::MerchantId,
        _status: common_enums::enums::RemittanceStatus,
        _limit: Option<i64>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<Remittance>, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittanceInterface::find_remittances_by_merchant_id_status not implemented for MockDb".to_string()))?
    }
}

#[async_trait::async_trait]
impl RemittancePaymentInterface for MockDb {
    type Error = StorageError;

    async fn find_remittance_payment_by_remittance_id(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance_id: &uuid::Uuid,
    ) -> CustomResult<RemittancePayment, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittancePaymentInterface::find_remittance_payment_by_remittance_id not implemented for MockDb".to_string()))?
    }

    async fn insert_remittance_payment(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance_payment: RemittancePayment,
    ) -> CustomResult<RemittancePayment, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittancePaymentInterface::insert_remittance_payment not implemented for MockDb".to_string()))?
    }

    async fn update_remittance_payment(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance_payment: RemittancePayment,
        _update: diesel_models::remittances::RemittancePaymentUpdate,
    ) -> CustomResult<RemittancePayment, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittancePaymentInterface::update_remittance_payment not implemented for MockDb".to_string()))?
    }
}

#[async_trait::async_trait]
impl RemittancePayoutInterface for MockDb {
    type Error = StorageError;

    async fn find_remittance_payout_by_remittance_id(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance_id: &uuid::Uuid,
    ) -> CustomResult<RemittancePayout, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittancePayoutInterface::find_remittance_payout_by_remittance_id not implemented for MockDb".to_string()))?
    }

    async fn insert_remittance_payout(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance_payout: RemittancePayout,
    ) -> CustomResult<RemittancePayout, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittancePayoutInterface::insert_remittance_payout not implemented for MockDb".to_string()))?
    }

    async fn update_remittance_payout(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        _remittance_payout: RemittancePayout,
        _update: diesel_models::remittances::RemittancePayoutUpdate,
    ) -> CustomResult<RemittancePayout, StorageError> {
        // TODO: Implement function for `MockDb`
        Err(StorageError::MockDbError("RemittancePayoutInterface::update_remittance_payout not implemented for MockDb".to_string()))?
    }
}