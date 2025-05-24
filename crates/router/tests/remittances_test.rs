//! Integration tests for remittances

#[cfg(feature = "remittances")]
mod remittance_tests {
    use serial_test::serial;
    use test_utils::{
        connector_auth::ConnectorAuthentication,
        fixtures::{self, ConnectorActions},
    };

    #[tokio::test]
    #[serial]
    async fn test_remittance_create_flow() {
        let conn = ConnectorAuthentication::new()
            .with_connector("wise".to_string())
            .with_merchant_id("test_merchant".to_string());
        
        let response = fixtures::create_remittance(
            conn,
            Some(fixtures::RemittanceCreateRequest {
                amount: 1000,
                source_currency: "USD".to_string(),
                destination_currency: "EUR".to_string(),
                sender_name: "John Doe".to_string(),
                beneficiary_name: "Jane Smith".to_string(),
                reference: "Test remittance".to_string(),
                ..Default::default()
            }),
        )
        .await
        .expect("Remittance creation should succeed");
        
        assert_eq!(response.status, "created");
    }
    
    #[tokio::test]
    #[serial]
    async fn test_remittance_payment_flow() {
        let conn = ConnectorAuthentication::new()
            .with_connector("wise".to_string())
            .with_merchant_id("test_merchant".to_string());
        
        // Create remittance
        let remittance = fixtures::create_remittance(
            conn.clone(),
            Some(fixtures::RemittanceCreateRequest {
                amount: 1000,
                source_currency: "USD".to_string(),
                destination_currency: "EUR".to_string(),
                sender_name: "John Doe".to_string(),
                beneficiary_name: "Jane Smith".to_string(),
                reference: "Test payment".to_string(),
                ..Default::default()
            }),
        )
        .await
        .expect("Remittance creation should succeed");
        
        // Pay remittance
        let payment_response = fixtures::pay_remittance(
            conn,
            &remittance.remittance_id,
            Some(fixtures::RemittancePayRequest {
                payment_method_data: Some(fixtures::PaymentMethodData::Card {
                    card_number: "4242424242424242".to_string(),
                    exp_month: "12".to_string(),
                    exp_year: "2025".to_string(),
                    cvv: "123".to_string(),
                }),
                ..Default::default()
            }),
        )
        .await
        .expect("Remittance payment should succeed");
        
        assert_eq!(payment_response.status, "payment_initiated");
    }
    
    #[tokio::test]
    #[serial]
    async fn test_remittance_quote() {
        let conn = ConnectorAuthentication::new()
            .with_connector("wise".to_string())
            .with_merchant_id("test_merchant".to_string());
        
        let quote = fixtures::get_remittance_quote(
            conn,
            Some(fixtures::RemittanceQuoteRequest {
                source_currency: "USD".to_string(),
                destination_currency: "EUR".to_string(),
                amount: 1000,
                ..Default::default()
            }),
        )
        .await
        .expect("Quote should succeed");
        
        assert!(quote.rate > 0.0);
        assert!(quote.destination_amount > 0);
    }
    
    #[tokio::test]
    #[serial]
    async fn test_remittance_cancel_with_refund() {
        let conn = ConnectorAuthentication::new()
            .with_connector("wise".to_string())
            .with_merchant_id("test_merchant".to_string());
        
        // Create and pay remittance
        let remittance = fixtures::create_and_pay_remittance(
            conn.clone(),
            Some(fixtures::RemittanceCreateRequest {
                amount: 1000,
                source_currency: "USD".to_string(),
                destination_currency: "EUR".to_string(),
                sender_name: "John Doe".to_string(),
                beneficiary_name: "Jane Smith".to_string(),
                reference: "Test cancel".to_string(),
                ..Default::default()
            }),
        )
        .await
        .expect("Remittance creation and payment should succeed");
        
        // Cancel remittance
        let cancel_response = fixtures::cancel_remittance(
            conn,
            &remittance.remittance_id,
        )
        .await
        .expect("Remittance cancellation should succeed");
        
        assert_eq!(cancel_response.status, "cancelled");
    }
}