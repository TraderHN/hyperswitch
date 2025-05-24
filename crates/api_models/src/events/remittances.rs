//! Métricas de eventos para API de remesas (v1 legacy / v2 moderno)

use common_utils::events::{ApiEventMetric, ApiEventsType};

//
// =====================================================================
// v1  (legacy – sólo si la build se compila con --features v1)
// =====================================================================
#[cfg(feature = "v1")]
mod v1 {
    use super::*;
    use crate::remittances::{
        RemittanceRequest          as RemittanceCreateRequest,
        RemittancesRetrieveRequest as RemittanceRetrieveRequest,
        RemittanceResponse,
        RemittanceListRequest      as RemittanceListConstraints,
        RemittanceListFilters      as RemittanceListFilterConstraints,
        RemittanceListFilters,
        RemittanceListResponse,
        // Las siguientes sólo existen en el stack v1:
        RemittanceActionRequest,
        RemittanceLinkInitiateRequest,
    };

    //----------------------------
    // Single‑resource operations
    //----------------------------
    impl ApiEventMetric for RemittanceRetrieveRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::Remittance {
                remittance_id: self.remittance_id.clone(),
            })
        }
    }
    impl ApiEventMetric for RemittanceCreateRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            self.remittance_id
                .as_deref()
                .map(|id| ApiEventsType::Remittance { remittance_id: id.to_owned() })
        }
    }
    impl ApiEventMetric for RemittanceResponse {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::Remittance {
                remittance_id: self.remittance_id.clone(),
            })
        }
    }
    impl ApiEventMetric for RemittanceActionRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::Remittance {
                remittance_id: self.remittance_id.clone(),
            })
        }
    }
    impl ApiEventMetric for RemittanceLinkInitiateRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::Remittance {
                remittance_id: self.remittance_id.clone(),
            })
        }
    }

    //----------------------------
    // List API
    //----------------------------
    impl ApiEventMetric for RemittanceListConstraints {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::ResourceListAPI)
        }
    }
    impl ApiEventMetric for RemittanceListFilterConstraints {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::ResourceListAPI)
        }
    }
    impl ApiEventMetric for RemittanceListFilters {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::ResourceListAPI)
        }
    }
    impl ApiEventMetric for RemittanceListResponse {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::ResourceListAPI)
        }
    }
}

//
// =====================================================================
// v2  (remittances feature moderno)
// =====================================================================
#[cfg(feature = "v2")]
mod v2 {
    use super::*;
    use crate::remittances::{
        RemittancePayRequest, RemittanceThirdPartyPayRequest,
        RemittanceListRequest, RemittanceListFilters, RemittanceListResponse,
        RemittanceRequest      as RemittanceCreateRequest,
        RemittanceResponse     as RemittanceCreateResponse,
        RemittancesRetrieveRequest,
    };

    //----------------------------
    // Single‑resource operations
    //----------------------------
    impl ApiEventMetric for RemittancesRetrieveRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::Remittance {
                remittance_id: self.remittance_id.clone(),
            })
        }
    }

    /// Durante la creación aún no existe ID → No se emite evento de resource.
    impl ApiEventMetric for RemittanceCreateRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            None
        }
    }

    impl ApiEventMetric for RemittanceCreateResponse {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::Remittance {
                remittance_id: self.remittance_id.clone(),
            })
        }
    }

    impl ApiEventMetric for RemittancePayRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            // El ID está en los path‑params, no en el body.
            None
        }
    }

    impl ApiEventMetric for RemittanceThirdPartyPayRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            None
        }
    }

    //----------------------------
    // List API
    //----------------------------
    impl ApiEventMetric for RemittanceListRequest {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::ResourceListAPI)
        }
    }
    impl ApiEventMetric for RemittanceListFilters {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::ResourceListAPI)
        }
    }
    impl ApiEventMetric for RemittanceListResponse {
        fn get_api_event_type(&self) -> Option<ApiEventsType> {
            Some(ApiEventsType::ResourceListAPI)
        }
    }
}
