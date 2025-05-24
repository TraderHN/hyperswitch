pub mod address;
pub mod api_keys;
pub mod authentication;
pub mod authorization;
pub mod blocklist;
pub mod blocklist_fingerprint;
pub mod blocklist_lookup;
pub mod business_profile;
pub mod callback_mapper;
pub mod capture;
pub mod configs;
pub mod customers;
pub mod dashboard_metadata;
pub mod dispute;
pub mod dynamic_routing_stats;
pub mod ephemeral_key;
pub mod events;
pub mod file;
pub mod fraud_check;
pub mod generic_link;
pub mod gsm;
pub mod health_check;
pub mod kafka_store;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod organization;
pub mod payment_link;
pub mod payment_method_session;
pub mod refund;
pub mod relay;
#[cfg(feature = "remittances")]
pub mod remittances;
pub mod reverse_lookup;
pub mod role;
pub mod routing_algorithm;
pub mod unified_translations;
pub mod user;
pub mod user_authentication_method;
pub mod user_key_store;
pub mod user_role;

use ::payment_methods::state::PaymentMethodsStorageInterface;
use common_utils::id_type;
use diesel_models::{
    fraud_check::{FraudCheck, FraudCheckUpdate},
    organization::{Organization, OrganizationNew, OrganizationUpdate},
};
use error_stack::ResultExt;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::payouts::{
    payout_attempt::PayoutAttemptInterface, payouts::PayoutsInterface,
};
use hyperswitch_domain_models::{
    cards_info::CardsInfoInterface,
    payment_methods::PaymentMethodInterface,
    payments::{payment_attempt::PaymentAttemptInterface, payment_intent::PaymentIntentInterface},
};
#[cfg(not(feature = "payouts"))]
use hyperswitch_domain_models::{PayoutAttemptInterface, PayoutsInterface};
use masking::PeekInterface;
use redis_interface::errors::RedisError;
use router_env::logger;
use storage_impl::{errors::StorageError, redis::kv_store::RedisConnInterface, MockDb};

pub use self::kafka_store::KafkaStore;
use self::{fraud_check::FraudCheckInterface, organization::OrganizationInterface};
#[cfg(feature = "remittances")]
use self::remittances::{RemittanceInterface, RemittancePaymentInterface, RemittancePayoutInterface};
pub use crate::{
    core::errors::{self, ProcessTrackerError},
    errors::CustomResult,
    services::{
        kafka::{KafkaError, KafkaProducer, MQResult},
        Store,
    },
    types::{
        domain,
        storage::{self},
        AccessToken,
    },
};

#[derive(PartialEq, Eq)]
pub enum StorageImpl {
    Postgresql,
    PostgresqlTest,
    Mock,
}

#[async_trait::async_trait]
pub trait StorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + address::AddressInterface
    + api_keys::ApiKeyInterface
    + blocklist_lookup::BlocklistLookupInterface
    + configs::ConfigInterface
    + capture::CaptureInterface
    + customers::CustomerInterface<Error = StorageError>
    + dashboard_metadata::DashboardMetadataInterface
    + dispute::DisputeInterface
    + ephemeral_key::EphemeralKeyInterface
    + ephemeral_key::ClientSecretInterface
    + events::EventInterface
    + file::FileMetadataInterface
    + FraudCheckInterface
    + locker_mock_up::LockerMockUpInterface
    + mandate::MandateInterface
    + merchant_account::MerchantAccountInterface
    + merchant_connector_account::ConnectorAccessToken
    + merchant_connector_account::MerchantConnectorAccountInterface
    + PaymentAttemptInterface<Error = StorageError>
    + PaymentIntentInterface<Error = StorageError>
    + PaymentMethodInterface<Error = StorageError>
    + blocklist::BlocklistInterface
    + blocklist_fingerprint::BlocklistFingerprintInterface
    + dynamic_routing_stats::DynamicRoutingStatsInterface
    + scheduler::SchedulerInterface
    + PayoutAttemptInterface<Error = StorageError>
    + PayoutsInterface<Error = StorageError>
    + refund::RefundInterface
    + reverse_lookup::ReverseLookupInterface
    + CardsInfoInterface<Error = StorageError>
    + merchant_key_store::MerchantKeyStoreInterface
    + MasterKeyInterface
    + payment_link::PaymentLinkInterface
    + RedisConnInterface
    + RequestIdStore
    + business_profile::ProfileInterface
    + routing_algorithm::RoutingAlgorithmInterface
    + gsm::GsmInterface
    + unified_translations::UnifiedTranslationsInterface
    + authorization::AuthorizationInterface
    + user::sample_data::BatchSampleDataInterface
    + health_check::HealthCheckDbInterface
    + user_authentication_method::UserAuthenticationMethodInterface
    + authentication::AuthenticationInterface
    + generic_link::GenericLinkInterface
    + relay::RelayInterface
    + user::theme::ThemeInterface
    + payment_method_session::PaymentMethodsSessionInterface
    + RemittanceStorageInterface
    + 'static
{
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface>;
    fn get_payment_methods_store(&self) -> Box<dyn PaymentMethodsStorageInterface>;
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)>;
}

/// Combined interface for all remittance-related storage operations
#[cfg(feature = "remittances")]
pub trait RemittanceStorageInterface:
    RemittanceInterface + RemittancePaymentInterface + RemittancePayoutInterface
{
}

/// Empty trait implementation when remittances feature is disabled
#[cfg(not(feature = "remittances"))]
pub trait RemittanceStorageInterface {}

/// Blanket implementation for any type that implements all remittance interfaces
#[cfg(feature = "remittances")]
impl<T> RemittanceStorageInterface for T where
    T: RemittanceInterface + RemittancePaymentInterface + RemittancePayoutInterface
{
}

/// Empty implementation when remittances feature is disabled
#[cfg(not(feature = "remittances"))]
impl<T> RemittanceStorageInterface for T {}

#[async_trait::async_trait]
pub trait GlobalStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + user::UserInterface
    + user_role::UserRoleInterface
    + user_key_store::UserKeyStoreInterface
    + role::RoleInterface
    + RedisConnInterface
    + 'static
{
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)>;
}

#[async_trait::async_trait]
pub trait AccountsStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + OrganizationInterface
    + merchant_account::MerchantAccountInterface
    + business_profile::ProfileInterface
    + merchant_connector_account::MerchantConnectorAccountInterface
    + merchant_key_store::MerchantKeyStoreInterface
    + dashboard_metadata::DashboardMetadataInterface
    + 'static
{
}

pub trait CommonStorageInterface:
    StorageInterface
    + GlobalStorageInterface
    + AccountsStorageInterface
    + PaymentMethodsStorageInterface
{
    fn get_storage_interface(&self) -> Box<dyn StorageInterface>;
    fn get_global_storage_interface(&self) -> Box<dyn GlobalStorageInterface>;
    fn get_accounts_storage_interface(&self) -> Box<dyn AccountsStorageInterface>;
}

pub trait MasterKeyInterface {
    fn get_master_key(&self) -> &[u8];
}

impl MasterKeyInterface for Store {
    fn get_master_key(&self) -> &[u8] {
        self.master_key().peek()
    }
}

/// Default dummy key for MockDb
impl MasterKeyInterface for MockDb {
    fn get_master_key(&self) -> &[u8] {
        &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]
    }
}

#[async_trait::async_trait]
impl StorageInterface for Store {
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface> {
        Box::new(self.clone())
    }
    fn get_payment_methods_store(&self) -> Box<dyn PaymentMethodsStorageInterface> {
        Box::new(self.clone())
    }

    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
impl GlobalStorageInterface for Store {
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

impl AccountsStorageInterface for Store {}

#[async_trait::async_trait]
impl StorageInterface for MockDb {
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface> {
        Box::new(self.clone())
    }
    fn get_payment_methods_store(&self) -> Box<dyn PaymentMethodsStorageInterface> {
        Box::new(self.clone())
    }

    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
impl GlobalStorageInterface for MockDb {
    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

impl AccountsStorageInterface for MockDb {}

impl CommonStorageInterface for MockDb {
    fn get_global_storage_interface(&self) -> Box<dyn GlobalStorageInterface> {
        Box::new(self.clone())
    }
    fn get_storage_interface(&self) -> Box<dyn StorageInterface> {
        Box::new(self.clone())
    }

    fn get_accounts_storage_interface(&self) -> Box<dyn AccountsStorageInterface> {
        Box::new(self.clone())
    }
}

impl CommonStorageInterface for Store {
    fn get_global_storage_interface(&self) -> Box<dyn GlobalStorageInterface> {
        Box::new(self.clone())
    }
    fn get_storage_interface(&self) -> Box<dyn StorageInterface> {
        Box::new(self.clone())
    }
    fn get_accounts_storage_interface(&self) -> Box<dyn AccountsStorageInterface> {
        Box::new(self.clone())
    }
}

pub trait RequestIdStore {
    fn add_request_id(&mut self, _request_id: String) {}
    fn get_request_id(&self) -> Option<String> {
        None
    }
}

impl RequestIdStore for MockDb {}

impl RequestIdStore for Store {
    fn add_request_id(&mut self, request_id: String) {
        self.request_id = Some(request_id)
    }

    fn get_request_id(&self) -> Option<String> {
        self.request_id.clone()
    }
}

pub async fn get_and_deserialize_key<T>(
    db: &dyn StorageInterface,
    key: &str,
    type_name: &'static str,
) -> CustomResult<T, RedisError>
where
    T: serde::de::DeserializeOwned,
{
    use common_utils::ext_traits::ByteSliceExt;

    let bytes = db.get_key(key).await?;
    bytes
        .parse_struct(type_name)
        .change_context(RedisError::JsonDeserializationFailed)
}

dyn_clone::clone_trait_object!(StorageInterface);
dyn_clone::clone_trait_object!(GlobalStorageInterface);
dyn_clone::clone_trait_object!(AccountsStorageInterface);

impl RequestIdStore for KafkaStore {
    fn add_request_id(&mut self, request_id: String) {
        self.diesel_store.add_request_id(request_id)
    }
}

#[async_trait::async_trait]
impl FraudCheckInterface for KafkaStore {
    async fn insert_fraud_check_response(
        &self,
        new: storage::FraudCheckNew,
    ) -> CustomResult<FraudCheck, StorageError> {
        let frm = self.diesel_store.insert_fraud_check_response(new).await?;
        if let Err(er) = self
            .kafka_producer
            .log_fraud_check(&frm, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message = "Failed to log analytics event for fraud check", error_message = ?er);
        }
        Ok(frm)
    }
    async fn update_fraud_check_response_with_attempt_id(
        &self,
        this: FraudCheck,
        fraud_check: FraudCheckUpdate,
    ) -> CustomResult<FraudCheck, StorageError> {
        let frm = self
            .diesel_store
            .update_fraud_check_response_with_attempt_id(this, fraud_check)
            .await?;
        if let Err(er) = self
            .kafka_producer
            .log_fraud_check(&frm, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for fraud check {frm:?}", error_message=?er)
        }
        Ok(frm)
    }
    async fn find_fraud_check_by_payment_id(
        &self,
        payment_id: id_type::PaymentId,
        merchant_id: id_type::MerchantId,
    ) -> CustomResult<FraudCheck, StorageError> {
        let frm = self
            .diesel_store
            .find_fraud_check_by_payment_id(payment_id, merchant_id)
            .await?;
        if let Err(er) = self
            .kafka_producer
            .log_fraud_check(&frm, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for fraud check {frm:?}", error_message=?er)
        }
        Ok(frm)
    }
    async fn find_fraud_check_by_payment_id_if_present(
        &self,
        payment_id: id_type::PaymentId,
        merchant_id: id_type::MerchantId,
    ) -> CustomResult<Option<FraudCheck>, StorageError> {
        let frm = self
            .diesel_store
            .find_fraud_check_by_payment_id_if_present(payment_id, merchant_id)
            .await?;

        if let Some(fraud_check) = frm.clone() {
            if let Err(er) = self
                .kafka_producer
                .log_fraud_check(&fraud_check, None, self.tenant_id.clone())
                .await
            {
                logger::error!(message="Failed to log analytics event for frm {frm:?}", error_message=?er);
            }
        }
        Ok(frm)
    }
}

#[async_trait::async_trait]
impl OrganizationInterface for KafkaStore {
    async fn insert_organization(
        &self,
        organization: OrganizationNew,
    ) -> CustomResult<Organization, StorageError> {
        self.diesel_store.insert_organization(organization).await
    }
    async fn find_organization_by_org_id(
        &self,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<Organization, StorageError> {
        self.diesel_store.find_organization_by_org_id(org_id).await
    }

    async fn update_organization_by_org_id(
        &self,
        org_id: &id_type::OrganizationId,
        update: OrganizationUpdate,
    ) -> CustomResult<Organization, StorageError> {
        self.diesel_store
            .update_organization_by_org_id(org_id, update)
            .await
    }
}

/// Implementations for remittances in KafkaStore
#[cfg(feature = "remittances")]
#[async_trait::async_trait]
impl RemittanceInterface for KafkaStore {
    async fn insert_remittance(
        &self,
        remittance: diesel_models::remittance::RemittanceNew,
    ) -> CustomResult<diesel_models::remittance::Remittance, StorageError> {
        let result = self.diesel_store.insert_remittance(remittance).await?;
        
        // Log to analytics if needed
        if let Err(er) = self
            .kafka_producer
            .log_remittance(&result, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message = "Failed to log analytics event for remittance", error_message = ?er);
        }
        
        Ok(result)
    }

    async fn find_remittance_by_id(
        &self,
        id: &uuid::Uuid,
    ) -> CustomResult<diesel_models::remittance::Remittance, StorageError> {
        self.diesel_store.find_remittance_by_id(id).await
    }

    async fn find_remittance_by_merchant_id_remittance_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        id: &uuid::Uuid,
    ) -> CustomResult<diesel_models::remittance::Remittance, StorageError> {
        self.diesel_store
            .find_remittance_by_merchant_id_remittance_id(merchant_id, id)
            .await
    }

    async fn update_remittance(
        &self,
        this: diesel_models::remittance::Remittance,
        remittance: diesel_models::remittance::RemittanceUpdate,
    ) -> CustomResult<diesel_models::remittance::Remittance, StorageError> {
        let result = self.diesel_store.update_remittance(this, remittance).await?;
        
        // Log to analytics if needed
        if let Err(er) = self
            .kafka_producer
            .log_remittance(&result, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message = "Failed to log analytics event for remittance update", error_message = ?er);
        }
        
        Ok(result)
    }

    async fn delete_remittance_by_merchant_id_remittance_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        id: &uuid::Uuid,
    ) -> CustomResult<bool, StorageError> {
        self.diesel_store
            .delete_remittance_by_merchant_id_remittance_id(merchant_id, id)
            .await
    }

    async fn filter_remittances_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &api_models::remittances::RemittanceListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<diesel_models::remittance::Remittance>, StorageError> {
        self.diesel_store
            .filter_remittances_by_constraints(merchant_id, filters, limit, offset)
            .await
    }

    async fn get_remittances_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &api_models::remittances::RemittanceListRequest,
    ) -> CustomResult<i64, StorageError> {
        self.diesel_store
            .get_remittances_count(merchant_id, filters)
            .await
    }

    async fn get_remittance_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(api_models::remittances::RemittanceStatus, i64)>, StorageError> {
        self.diesel_store
            .get_remittance_status_with_count(merchant_id, profile_id_list, time_range)
            .await
    }

    async fn find_remittances_by_merchant_id_status(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        status: api_models::remittances::RemittanceStatus,
        limit: i64,
    ) -> CustomResult<Vec<diesel_models::remittance::Remittance>, StorageError> {
        self.diesel_store
            .find_remittances_by_merchant_id_status(merchant_id, status, limit)
            .await
    }
}

#[cfg(feature = "remittances")]
#[async_trait::async_trait]
impl RemittancePaymentInterface for KafkaStore {
    async fn insert_remittance_payment(
        &self,
        remittance_payment: diesel_models::remittance::RemittancePaymentNew,
    ) -> CustomResult<diesel_models::remittance::RemittancePayment, StorageError> {
        self.diesel_store
            .insert_remittance_payment(remittance_payment)
            .await
    }

    async fn find_remittance_payment_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
    ) -> CustomResult<diesel_models::remittance::RemittancePayment, StorageError> {
        self.diesel_store
            .find_remittance_payment_by_remittance_id(remittance_id)
            .await
    }

    async fn update_remittance_payment_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
        remittance_payment: diesel_models::remittance::RemittancePaymentUpdate,
    ) -> CustomResult<diesel_models::remittance::RemittancePayment, StorageError> {
        self.diesel_store
            .update_remittance_payment_by_remittance_id(remittance_id, remittance_payment)
            .await
    }

    async fn find_remittance_payment_by_payment_id(
        &self,
        payment_id: &str,
    ) -> CustomResult<diesel_models::remittance::RemittancePayment, StorageError> {
        self.diesel_store
            .find_remittance_payment_by_payment_id(payment_id)
            .await
    }
}

#[cfg(feature = "remittances")]
#[async_trait::async_trait]
impl RemittancePayoutInterface for KafkaStore {
    async fn insert_remittance_payout(
        &self,
        remittance_payout: diesel_models::remittance::RemittancePayoutNew,
    ) -> CustomResult<diesel_models::remittance::RemittancePayout, StorageError> {
        self.diesel_store
            .insert_remittance_payout(remittance_payout)
            .await
    }

    async fn find_remittance_payout_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
    ) -> CustomResult<diesel_models::remittance::RemittancePayout, StorageError> {
        self.diesel_store
            .find_remittance_payout_by_remittance_id(remittance_id)
            .await
    }

    async fn update_remittance_payout_by_remittance_id(
        &self,
        remittance_id: &uuid::Uuid,
        remittance_payout: diesel_models::remittance::RemittancePayoutUpdate,
    ) -> CustomResult<diesel_models::remittance::RemittancePayout, StorageError> {
        self.diesel_store
            .update_remittance_payout_by_remittance_id(remittance_id, remittance_payout)
            .await
    }

    async fn find_remittance_payout_by_payout_id(
        &self,
        payout_id: &str,
    ) -> CustomResult<diesel_models::remittance::RemittancePayout, StorageError> {
        self.diesel_store
            .find_remittance_payout_by_payout_id(payout_id)
            .await
    }
}