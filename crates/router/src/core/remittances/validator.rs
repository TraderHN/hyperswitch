//! Business validation functions for remittances

use common_utils::errors::CustomResult;
use error_stack::report;
use time::Date;

use crate::{
    core::errors,
    types::{api, domain, storage},
};

/// Validate remittance can be paid
pub fn validate_remittance_payable(
    remittance: &storage::Remittance,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Check status
    match remittance.status.as_str() {
        "created" => Ok(()),
        "payment_initiated" => Err(report!(errors::ApiErrorResponse::DuplicateRequest {
            message: "Payment already initiated".to_string()
        })),
        "payment_processed" | "payout_initiated" | "completed" => {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Remittance already paid".to_string()
            }))
        }
        "failed" => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Cannot pay a failed remittance".to_string()
        })),
        "cancelled" => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Cannot pay a cancelled remittance".to_string()
        })),
        _ => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid remittance status".to_string()
        })),
    }
}

/// Validate remittance can be updated
pub fn validate_remittance_updatable(
    remittance: &storage::Remittance,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Only allow updates in created state
    if remittance.status != "created" {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Can only update remittance in created state".to_string()
        }));
    }
    
    Ok(())
}

/// Validate remittance can be cancelled
pub fn validate_remittance_cancellable(
    remittance: &storage::Remittance,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Check status
    match remittance.status.as_str() {
        "created" | "payment_initiated" | "payment_processed" => Ok(()),
        "payout_initiated" => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Cannot cancel remittance after payout initiated".to_string()
        })),
        "completed" => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Cannot cancel completed remittance".to_string()
        })),
        "failed" => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Remittance already failed".to_string()
        })),
        "cancelled" => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Remittance already cancelled".to_string()
        })),
        _ => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid remittance status".to_string()
        })),
    }
}

/// Validate currency support
pub fn validate_currency_support(
    profile: &domain::Profile,
    source_currency: &api_models::enums::Currency,
    destination_currency: &api_models::enums::Currency,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // TODO: Check profile configuration for supported currency pairs
    // For now, allow all currencies
    
    if source_currency == destination_currency {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Source and destination currencies must be different".to_string()
        }));
    }
    
    Ok(())
}

/// Validate remittance date
pub fn validate_remittance_date(date_str: &str) -> CustomResult<(), errors::ApiErrorResponse> {
    // Parse date
    let date = Date::parse(
        date_str,
        &time::format_description::well_known::Iso8601::DATE,
    )
    .map_err(|_| {
        report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "remittance_date".to_string(),
            expected_format: "YYYY-MM-DD".to_string(),
        })
    })?;
    
    // Check not in future
    let today = time::OffsetDateTime::now_utc().date();
    if date > today {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Remittance date cannot be in the future".to_string()
        }));
    }
    
    Ok(())
}

/// Validate list request
pub fn validate_list_request(
    req: &api::remittances::RemittanceListRequest,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Validate limit
    if let Some(limit) = req.limit {
        if limit == 0 || limit > 100 {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Limit must be between 1 and 100".to_string()
            }));
        }
    }
    
    // Validate time range
    if let Some(time_range) = &req.time_range {
        if let Some(end_time) = time_range.end_time {
            if end_time < time_range.start_time {
                return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "End time must be after start time".to_string()
                }));
            }
        }
    }
    
    Ok(())
}

/// Validate sync request
pub fn validate_sync_request(
    req: &api::remittances::RemittanceSyncRequest,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Validate time range if provided
    if let Some(time_range) = &req.time_range {
        if let Some(end_time) = time_range.end_time {
            if end_time < time_range.start_time {
                return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "End time must be after start time".to_string()
                }));
            }
        }
        
        // Don't allow syncing remittances older than 30 days
        let thirty_days_ago = common_utils::date_time::now() - time::Duration::days(30);
        if time_range.start_time < thirty_days_ago {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Cannot sync remittances older than 30 days".to_string()
            }));
        }
    }
    
    Ok(())
}

/// Validate amount
pub fn validate_amount(amount: i64) -> CustomResult<(), errors::ApiErrorResponse> {
    if amount <= 0 {
        return Err(report!(errors::ApiErrorResponse::InvalidAmount));
    }
    
    // Check maximum amount (example: $1M)
    if amount > 100_000_000 {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Amount exceeds maximum allowed".to_string()
        }));
    }
    
    Ok(())
}

/// Validate beneficiary details
pub fn validate_beneficiary_details(
    details: &api::remittances::BeneficiaryDetails,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Validate name
    if details.name.trim().is_empty() {
        return Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "beneficiary_details.name".to_string()
        }));
    }
    
    // Validate payout method exists
    if details.payout_details.is_none() {
        return Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "beneficiary_details.payout_details".to_string()
        }));
    }
    
    // TODO: Add more validations based on payout method type
    
    Ok(())
}

/// Validate sender details
pub fn validate_sender_details(
    details: &api::remittances::SenderDetails,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Validate name
    if details.name.trim().is_empty() {
        return Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "sender_details.name".to_string()
        }));
    }
    
    // Either customer_id or email should be present
    if details.customer_id.is_none() && details.email.is_none() {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "Either customer_id or email must be provided".to_string()
        }));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_amount() {
        assert!(validate_amount(100).is_ok());
        assert!(validate_amount(0).is_err());
        assert!(validate_amount(-100).is_err());
        assert!(validate_amount(100_000_001).is_err());
    }
    
    #[test]
    fn test_validate_remittance_date() {
        let today = time::OffsetDateTime::now_utc().date();
        assert!(validate_remittance_date(&today.to_string()).is_ok());
        
        let yesterday = today - time::Duration::days(1);
        assert!(validate_remittance_date(&yesterday.to_string()).is_ok());
        
        let tomorrow = today + time::Duration::days(1);
        assert!(validate_remittance_date(&tomorrow.to_string()).is_err());
        
        assert!(validate_remittance_date("invalid-date").is_err());
    }
}