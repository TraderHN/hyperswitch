use base64::Engine;
use common_enums::{enums, FutureUsage};
use common_utils::{consts, pii};
use hyperswitch_domain_models::{
    payment_method_data::{ApplePayWalletData, GooglePayWalletData},
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ApplePayPredecryptData, ConnectorAuthType,
        ConnectorResponseData, ErrorResponse, PaymentMethodToken, RouterData,
    },
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        ResponseId,
    },
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{api, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    constants,
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{
        self, AddressDetailsData, ApplePayDecrypt, CardData, PaymentsAuthorizeRequestData,
        PaymentsSetupMandateRequestData, PaymentsSyncRequestData, RecurringMandateData,
        RouterData as OtherRouterData,
    },
};

/// Tipo que encapsula la info de autenticación LAFISE (antes Bank of America).
pub struct LafiseAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for LafiseAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

/// Estructura para agrupar la cantidad (convertida a string) y la data adjunta.
pub struct LafiseRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, api_models::enums::Currency, i64, T)>
    for LafiseRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &api::CurrencyUnit,
            api_models::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

/// Estructura general de request de pagos LAFISE (antes BankOfAmericaPaymentsRequest).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafisePaymentsRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    consumer_authentication_information: Option<LafiseConsumerAuthInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

/// Define la lógica de procesamiento (capture, tokenization, etc.).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformation {
    action_list: Option<Vec<LafiseActionsList>>,
    action_token_types: Option<Vec<LafiseActionsTokenType>>,
    authorization_options: Option<LafiseAuthorizationOptions>,
    commerce_indicator: String,
    capture: Option<bool>,
    capture_options: Option<CaptureOptions>,
    payment_solution: Option<String>,
}

/// Lista de “acciones” a realizar (TokenCreate, etc.) para LAFISE.
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LafiseActionsList {
    TokenCreate,
}

/// Tipo de token a crear (instrumento de pago, Customer…).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LafiseActionsTokenType {
    PaymentInstrument,
    Customer,
}

/// Opciones de Autorización en LAFISE (antes BankOfAmericaAuthorizationOptions).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseAuthorizationOptions {
    initiator: Option<LafisePaymentInitiator>,
    merchant_intitiated_transaction: Option<MerchantInitiatedTransaction>,
}

/// Representa la entidad que inicia el pago (Customer, etc.).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafisePaymentInitiator {
    #[serde(rename = "type")]
    initiator_type: Option<LafisePaymentInitiatorTypes>,
    credential_stored_on_file: Option<bool>,
    stored_credential_used: Option<bool>,
}

/// Tipos de “Initiator”.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LafisePaymentInitiatorTypes {
    Customer,
}

/// Estructura que guarda la razón de un MerchantInitiatedTransaction, etc.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantInitiatedTransaction {
    reason: Option<String>,
    original_authorized_amount: Option<String>,
}

/// Información de merchant definida.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDefinedInformation {
    key: u8,
    value: String,
}

/// Información de autenticación de consumidores LAFISE.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseConsumerAuthInformation {
    ucaf_collection_indicator: Option<String>,
    cavv: Option<String>,
    ucaf_authentication_data: Option<Secret<String>>,
    xid: Option<String>,
    directory_server_transaction_id: Option<Secret<String>>,
    specification_version: Option<String>,
}

/// Opciones de captura, si se hace.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOptions {
    capture_sequence_number: u32,
    total_capture_count: u32,
}

/// Estructura de instrumento de pago con ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LafisePaymentInstrument {
    pub id: Secret<String>,
}

/// Info de tarjeta.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentInformation {
    card: Card,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPaymentInformation {
    fluid_data: FluidData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayTokenizedCard {
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayTokenPaymentInformation {
    fluid_data: FluidData,
    tokenized_card: ApplePayTokenizedCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayPaymentInformation {
    tokenized_card: TokenizedCard,
}

/// Enum “PaymentInformation” final: tarjeta, google, apple, etc.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentInformation {
    Cards(Box<CardPaymentInformation>),
    GooglePay(Box<GooglePayPaymentInformation>),
    ApplePay(Box<ApplePayPaymentInformation>),
    ApplePayToken(Box<ApplePayTokenPaymentInformation>),
    MandatePayment(Box<MandatePaymentInformation>),
}

/// Si se usa un MandatePayment, su info: PaymentInstrument con ID.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatePaymentInformation {
    payment_instrument: LafisePaymentInstrument,
}

/// Datos de tarjeta para el request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Secret<String>,
    #[serde(rename = "type")]
    card_type: Option<String>,
}

/// Datos tokenizados en ApplePay
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCard {
    number: Secret<String>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    cryptogram: Secret<String>,
    transaction_type: TransactionType,
}

/// Contiene la data codificada (`fluid_data`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FluidData {
    value: Secret<String>,
}

/// Estructura con el monto y la info de facturación.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationWithBill {
    amount_details: Amount,
    bill_to: Option<BillTo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total_amount: String,
    currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillTo {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address1: Option<Secret<String>>,
    locality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    administrative_area: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    postal_code: Option<Secret<String>>,
    country: Option<enums::CountryAlpha2>,
    email: pii::Email,
}

/// ClientReferenceInformation: un “reference code” que se usa en la request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReferenceInformation {
    code: Option<String>,
}

/// Convierte un `metadata: Value` a vector de MerchantDefinedInformation.
fn convert_metadata_to_merchant_defined_info(metadata: Value) -> Vec<MerchantDefinedInformation> {
    let hashmap: std::collections::BTreeMap<String, Value> =
        serde_json::from_str(&metadata.to_string()).unwrap_or_default();
    let mut vector = Vec::new();
    let mut iter = 1;
    for (key, value) in hashmap {
        vector.push(MerchantDefinedInformation {
            key: iter,
            value: format!("{key}={value}"),
        });
        iter += 1;
    }
    vector
}

/// Genera la struct `BillTo` en base a la Address + email, si existen.
fn build_bill_to(
    address_details: Option<&hyperswitch_domain_models::address::Address>,
    email: pii::Email,
) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
    let default_address = BillTo {
        first_name: None,
        last_name: None,
        address1: None,
        locality: None,
        administrative_area: None,
        postal_code: None,
        country: None,
        email: email.clone(),
    };
    Ok(address_details
        .and_then(|addr| {
            addr.address.as_ref().map(|addr| BillTo {
                first_name: addr.first_name.clone(),
                last_name: addr.last_name.clone(),
                address1: addr.line1.clone(),
                locality: addr.city.clone(),
                administrative_area: addr.to_state_code_as_optional().ok().flatten(),
                postal_code: addr.zip.clone(),
                country: addr.country,
                email,
            })
        })
        .unwrap_or(default_address))
}

/// Mapeo simple de card network a un “type” string.  
fn get_lafise_card_type(card_network: common_enums::CardNetwork) -> Option<&'static str> {
    match card_network {
        common_enums::CardNetwork::Visa => Some("001"),
        common_enums::CardNetwork::Mastercard => Some("002"),
        common_enums::CardNetwork::AmericanExpress => Some("003"),
        common_enums::CardNetwork::JCB => Some("007"),
        common_enums::CardNetwork::DinersClub => Some("005"),
        common_enums::CardNetwork::Discover => Some("004"),
        common_enums::CardNetwork::CartesBancaires => Some("006"),
        common_enums::CardNetwork::UnionPay => Some("062"),
        //"042" is the type code for Masetro Cards(International). For Maestro Cards(UK-Domestic) the mapping should be "024"
        common_enums::CardNetwork::Maestro => Some("042"),
        common_enums::CardNetwork::Interac
        | common_enums::CardNetwork::RuPay
        | common_enums::CardNetwork::Star
        | common_enums::CardNetwork::Accel
        | common_enums::CardNetwork::Pulse
        | common_enums::CardNetwork::Nyce => None,
    }
}

/// Algún “PaymentSolution” (ej: ApplePay) => valor string “001” / “012”.
#[derive(Debug, Serialize)]
pub enum PaymentSolution {
    ApplePay,
    GooglePay,
}
impl From<PaymentSolution> for String {
    fn from(solution: PaymentSolution) -> Self {
        match solution {
            PaymentSolution::ApplePay => "001",
            PaymentSolution::GooglePay => "012",
        }
        .to_string()
    }
}

/// Tipo enumerado para transactionType en ApplePay (por ejemplo).
#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "1")]
    ApplePay,
}

// ============ PROCESSING INFORMATION UTILS ============== //

impl
    TryFrom<(
        &LafiseRouterData<&PaymentsAuthorizeRouterData>,
        Option<PaymentSolution>,
        Option<String>,
    )> for ProcessingInformation
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, solution, network): (
            &LafiseRouterData<&PaymentsAuthorizeRouterData>,
            Option<PaymentSolution>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let (action_list, action_token_types, authorization_options) =
            if item.router_data.request.setup_future_usage == Some(FutureUsage::OffSession)
                && (item.router_data.request.customer_acceptance.is_some()
                    || item
                        .router_data
                        .request
                        .setup_mandate_details
                        .clone()
                        .is_some_and(|mandate_details| {
                            mandate_details.customer_acceptance.is_some()
                        }))
            {
                // Genera la config de “token creation” + “authorization_options”
                get_lafise_mandate_action_details()
            } else if item.router_data.request.connector_mandate_id().is_some() {
                // Se asume que es un Payment Initiated con un existing Mandate
                let original_amount = item
                    .router_data
                    .get_recurring_mandate_payment_data()?
                    .get_original_payment_amount()?;
                let original_currency = item
                    .router_data
                    .get_recurring_mandate_payment_data()?
                    .get_original_payment_currency()?;
                (
                    None,
                    None,
                    Some(LafiseAuthorizationOptions {
                        initiator: None,
                        merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                            reason: None,
                            original_authorized_amount: Some(utils::get_amount_as_string(
                                &api::CurrencyUnit::Base,
                                original_amount,
                                original_currency,
                            )?),
                        }),
                    }),
                )
            } else {
                (None, None, None)
            };

        // Determina commerceIndicator según sea Visa, Master, etc.
        let commerce_indicator = get_commerce_indicator(network);

        Ok(Self {
            capture: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            payment_solution: solution.map(String::from),
            action_list,
            action_token_types,
            authorization_options,
            capture_options: None,
            commerce_indicator,
        })
    }
}

/// A partir de un “network” (ej “Visa”) => string (“internet”, “spa” en Master...).  
fn get_commerce_indicator(network: Option<String>) -> String {
    match network {
        Some(card_network) => match card_network.to_lowercase().as_str() {
            "amex" => "aesk",
            "discover" => "dipb",
            "mastercard" => "spa",
            "visa" => "internet",
            _ => "internet",
        },
        None => "internet",
    }
    .to_string()
}

/// Manda “TokenCreate” + “PaymentInstrument / Customer” + “AuthorizationOptions”.
fn get_lafise_mandate_action_details() -> (
    Option<Vec<LafiseActionsList>>,
    Option<Vec<LafiseActionsTokenType>>,
    Option<LafiseAuthorizationOptions>,
) {
    (
        Some(vec![LafiseActionsList::TokenCreate]),
        Some(vec![
            LafiseActionsTokenType::PaymentInstrument,
            LafiseActionsTokenType::Customer,
        ]),
        Some(LafiseAuthorizationOptions {
            initiator: Some(LafisePaymentInitiator {
                initiator_type: Some(LafisePaymentInitiatorTypes::Customer),
                credential_stored_on_file: Some(true),
                stored_credential_used: None,
            }),
            merchant_intitiated_transaction: None,
        }),
    )
}

// ============ CLIENT REFERENCE ============== //

impl From<&LafiseRouterData<&PaymentsAuthorizeRouterData>> for ClientReferenceInformation {
    fn from(item: &LafiseRouterData<&PaymentsAuthorizeRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

impl From<&SetupMandateRouterData> for ClientReferenceInformation {
    fn from(item: &SetupMandateRouterData) -> Self {
        Self {
            code: Some(item.connector_request_reference_id.clone()),
        }
    }
}

// ============ ORDER WITH BILL ============== //

impl
    From<(
        &LafiseRouterData<&PaymentsAuthorizeRouterData>,
        Option<BillTo>,
    )> for OrderInformationWithBill
{
    fn from(
        (item, bill_to): (
            &LafiseRouterData<&PaymentsAuthorizeRouterData>,
            Option<BillTo>,
        ),
    ) -> Self {
        Self {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency: item.router_data.request.currency,
            },
            bill_to,
        }
    }
}

/// Mapeo del SetupMandateRouterData => un OrderInformationWithBill con “0” de amount.
impl TryFrom<&SetupMandateRouterData> for OrderInformationWithBill {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        let email = item.request.get_email()?;
        let bill_to = build_bill_to(item.get_optional_billing(), email)?;
        Ok(Self {
            amount_details: Amount {
                total_amount: "0".to_string(),
                currency: item.request.currency,
            },
            bill_to: Some(bill_to),
        })
    }
}

// ============ PaymentInformation (Card, ApplePay, GooglePay, MandatePayment) ============== //

impl
    TryFrom<&hyperswitch_domain_models::payment_method_data::Card> for PaymentInformation
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        ccard: &hyperswitch_domain_models::payment_method_data::Card,
    ) -> Result<Self, Self::Error> {
        let card_type = match ccard.card_network.clone().and_then(get_lafise_card_type) {
            Some(card_network) => Some(card_network.to_string()),
            None => ccard.get_card_issuer().ok().map(String::from),
        };
        Ok(Self::Cards(Box::new(CardPaymentInformation {
            card: Card {
                number: ccard.card_number.clone(),
                expiration_month: ccard.card_exp_month.clone(),
                expiration_year: ccard.card_exp_year.clone(),
                security_code: ccard.card_cvc.clone(),
                card_type,
            },
        })))
    }
}

/// Apple Pay + “Predecrypt Data”.
impl TryFrom<&Box<ApplePayPredecryptData>> for PaymentInformation {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(apple_pay_data: &Box<ApplePayPredecryptData>) -> Result<Self, Self::Error> {
        let expiration_month = apple_pay_data.get_expiry_month()?;
        let expiration_year = apple_pay_data.get_four_digit_expiry_year()?;
        Ok(Self::ApplePay(Box::new(ApplePayPaymentInformation {
            tokenized_card: TokenizedCard {
                number: apple_pay_data.application_primary_account_number.clone(),
                cryptogram: apple_pay_data.payment_data.online_payment_cryptogram.clone(),
                transaction_type: TransactionType::ApplePay,
                expiration_year,
                expiration_month,
            },
        })))
    }
}

/// Apple Pay (Token form).
impl From<&ApplePayWalletData> for PaymentInformation {
    fn from(apple_pay_data: &ApplePayWalletData) -> Self {
        Self::ApplePayToken(Box::new(ApplePayTokenPaymentInformation {
            fluid_data: FluidData {
                value: Secret::from(apple_pay_data.payment_data.clone()),
            },
            tokenized_card: ApplePayTokenizedCard {
                transaction_type: TransactionType::ApplePay,
            },
        }))
    }
}

/// Google Pay => crea `GooglePayPaymentInformation` con `fluid_data`.
impl From<&GooglePayWalletData> for PaymentInformation {
    fn from(google_pay_data: &GooglePayWalletData) -> Self {
        Self::GooglePay(Box::new(GooglePayPaymentInformation {
            fluid_data: FluidData {
                value: Secret::from(
                    consts::BASE64_ENGINE.encode(google_pay_data.tokenization_data.token.clone()),
                ),
            },
        }))
    }
}

// ========== Adapta la request “SetupMandateRouterData” + (CardData o Apple Pay) => LafisePaymentsRequest ========== //

impl TryFrom<(&SetupMandateRouterData, hyperswitch_domain_models::payment_method_data::Card)>
    for LafisePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (&SetupMandateRouterData, hyperswitch_domain_models::payment_method_data::Card),
    ) -> Result<Self, Self::Error> {
        let order_information = OrderInformationWithBill::try_from(item)?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item.request.metadata.clone().map(|metadata| {
            convert_metadata_to_merchant_defined_info(metadata.peek().to_owned())
        });
        let payment_information = PaymentInformation::try_from(&ccard)?;
        let processing_information = ProcessingInformation::try_from((None, None))?;

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: None,
            merchant_defined_information,
        })
    }
}

impl TryFrom<(&SetupMandateRouterData, ApplePayWalletData)> for LafisePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, apple_pay_data): (&SetupMandateRouterData, ApplePayWalletData),
    ) -> Result<Self, Self::Error> {
        let order_information = OrderInformationWithBill::try_from(item)?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item.request.metadata.clone().map(|metadata| {
            convert_metadata_to_merchant_defined_info(metadata.peek().to_owned())
        });
        let payment_information = match item.payment_method_token.clone() {
            Some(payment_method_token) => match payment_method_token {
                PaymentMethodToken::GooglePayDecrypt(_) => {
    Err(unimplemented_payment_method!("GooglePayDecrypt", "Lafise"))?
}

                PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                    PaymentInformation::try_from(&decrypt_data)?
                }
                PaymentMethodToken::Token(_) => Err(unimplemented_payment_method!(
                    "Apple Pay",
                    "Manual",
                    "Lafise"
                ))?,
                PaymentMethodToken::PazeDecrypt(_) => {
                    Err(unimplemented_payment_method!("Paze", "Lafise"))?
                }
            },
            None => PaymentInformation::from(&apple_pay_data),
        };
        let processing_information =
            ProcessingInformation::try_from((Some(PaymentSolution::ApplePay), Some(apple_pay_data.payment_method.network.clone())))?;

        let ucaf_collection_indicator = match apple_pay_data
            .payment_method
            .network
            .to_lowercase()
            .as_str()
        {
            "mastercard" => Some("2".to_string()),
            _ => None,
        };

        let consumer_authentication_information = Some(LafiseConsumerAuthInformation {
            ucaf_collection_indicator,
            cavv: None,
            ucaf_authentication_data: None,
            xid: None,
            directory_server_transaction_id: None,
            specification_version: None,
        });

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            merchant_defined_information,
            consumer_authentication_information,
        })
    }
}

impl TryFrom<(&SetupMandateRouterData, GooglePayWalletData)> for LafisePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, google_pay_data): (&SetupMandateRouterData, GooglePayWalletData),
    ) -> Result<Self, Self::Error> {
        let order_information = OrderInformationWithBill::try_from(item)?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item.request.metadata.clone().map(|metadata| {
            convert_metadata_to_merchant_defined_info(metadata.peek().to_owned())
        });
        let payment_information = PaymentInformation::from(&google_pay_data);
        let processing_information =
            ProcessingInformation::try_from((Some(PaymentSolution::GooglePay), None))?;

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: None,
            merchant_defined_information,
        })
    }
}

impl TryFrom<(Option<PaymentSolution>, Option<String>)> for ProcessingInformation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (solution, network): (Option<PaymentSolution>, Option<String>),
    ) -> Result<Self, Self::Error> {
        let (action_list, action_token_types, authorization_options) =
            get_lafise_mandate_action_details();
        let commerce_indicator = get_commerce_indicator(network);

        Ok(Self {
            capture: Some(false),
            capture_options: None,
            action_list,
            action_token_types,
            authorization_options,
            commerce_indicator,
            payment_solution: solution.map(String::from),
        })
    }
}

// ========== PaymentInformation “authorize” con MandatePayment ========== //

impl TryFrom<(&LafiseRouterData<&PaymentsAuthorizeRouterData>, String)> for LafisePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, connector_mandate_id): (&LafiseRouterData<&PaymentsAuthorizeRouterData>, String),
    ) -> Result<Self, Self::Error> {
        let processing_information = ProcessingInformation::try_from((item, None, None))?;
        let payment_instrument = LafisePaymentInstrument {
            id: connector_mandate_id.into(),
        };
        let bill_to = item
            .router_data
            .request
            .get_email()
            .ok()
            .and_then(|email| build_bill_to(item.router_data.get_optional_billing(), email).ok());

        let order_information = OrderInformationWithBill::from((item, bill_to));
        let payment_information = PaymentInformation::MandatePayment(Box::new(MandatePaymentInformation {
            payment_instrument,
        }));
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            merchant_defined_information,
            consumer_authentication_information: None,
        })
    }
}

// ========== Respuestas ========== //

/// Equivalente a “BankofamericaPaymentStatus”; define la enumeración de estado en LAFISE.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LafisePaymentStatus {
    Authorized,
    Succeeded,
    Failed,
    Voided,
    Reversed,
    Pending,
    Declined,
    Rejected,
    Challenge,
    AuthorizedPendingReview,
    AuthorizedRiskDeclined,
    Transmitted,
    InvalidRequest,
    ServerError,
    PendingAuthentication,
    PendingReview,
    Accepted,
    Cancelled,
}

/// Mapeo del PaymentStatus LAFISE => AttemptStatus genérico.
fn map_lafise_attempt_status(
    (status, auto_capture): (LafisePaymentStatus, bool),
) -> enums::AttemptStatus {
    match status {
        LafisePaymentStatus::Authorized | LafisePaymentStatus::AuthorizedPendingReview => {
            if auto_capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        LafisePaymentStatus::Pending => {
            if auto_capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Pending
            }
        }
        LafisePaymentStatus::Succeeded | LafisePaymentStatus::Transmitted => {
            enums::AttemptStatus::Charged
        }
        LafisePaymentStatus::Voided
        | LafisePaymentStatus::Reversed
        | LafisePaymentStatus::Cancelled => enums::AttemptStatus::Voided,
        LafisePaymentStatus::Failed
        | LafisePaymentStatus::Declined
        | LafisePaymentStatus::AuthorizedRiskDeclined
        | LafisePaymentStatus::InvalidRequest
        | LafisePaymentStatus::Rejected
        | LafisePaymentStatus::ServerError => enums::AttemptStatus::Failure,
        LafisePaymentStatus::PendingAuthentication => enums::AttemptStatus::AuthenticationPending,
        LafisePaymentStatus::PendingReview
        | LafisePaymentStatus::Challenge
        | LafisePaymentStatus::Accepted => enums::AttemptStatus::Pending,
    }
}

/// Respuesta principal a un “Authorize”.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LafisePaymentsResponse {
    ClientReferenceInformation(Box<LafiseClientReferenceResponse>),
    ErrorInformation(Box<LafiseErrorInformationResponse>),
}

/// Respuesta principal a un “SetupMandate”.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LafiseSetupMandatesResponse {
    ClientReferenceInformation(Box<LafiseClientReferenceResponse>),
    ErrorInformation(Box<LafiseErrorInformationResponse>),
}

/// Estructura contenedora (antes BankOfAmericaClientReferenceResponse).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseClientReferenceResponse {
    pub id: String,
    pub status: LafisePaymentStatus,
    pub client_reference_information: ClientReferenceInformation,
    pub processor_information: Option<ClientProcessorInformation>,
    pub processing_information: Option<ProcessingInformationResponse>,
    pub payment_information: Option<PaymentInformationResponse>,
    pub payment_insights_information: Option<PaymentInsightsInformation>,
    pub risk_information: Option<ClientRiskInformation>,
    pub token_information: Option<LafiseTokenInformation>,
    pub error_information: Option<LafiseErrorInformation>,
    pub issuer_information: Option<IssuerInformation>,
    pub sender_information: Option<SenderInformation>,
    pub payment_account_information: Option<PaymentAccountInformation>,
    pub reconciliation_id: Option<String>,
    pub consumer_authentication_information: Option<ConsumerAuthenticationInformation>,
}

/// Info de autenticación 3DS, etc.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumerAuthenticationInformation {
    pub eci_raw: Option<String>,
    pub eci: Option<String>,
    pub acs_transaction_id: Option<String>,
    pub cavv: Option<String>,
}

/// Estructura de PaymentSolutionResponse, etc. LAFISE (análogo a la que había).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SenderInformation {
    pub payment_information: Option<PaymentInformationResponse>,
}

/// PaymentInsights + rules.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInsightsInformation {
    pub response_insights: Option<ResponseInsights>,
    pub rule_results: Option<RuleResults>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseInsights {
    pub category_code: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleResults {
    pub id: Option<String>,
    pub decision: Option<String>,
}

/// PaymentInformationResponse: card, bin, issuer, etc.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInformationResponse {
    pub tokenized_card: Option<CardResponseObject>,
    pub customer: Option<CustomerResponseObject>,
    pub card: Option<CardResponseObject>,
    pub scheme: Option<String>,
    pub bin: Option<String>,
    pub account_type: Option<String>,
    pub issuer: Option<String>,
    pub bin_country: Option<enums::CountryAlpha2>,
}

/// Objeto “customerResponseObject”.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerResponseObject {
    pub customer_id: Option<String>,
}

/// Estructura PaymentAccountInformation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAccountInformation {
    pub card: Option<PaymentAccountCardInformation>,
    pub features: Option<PaymentAccountFeatureInformation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAccountFeatureInformation {
    pub health_card: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAccountCardInformation {
    #[serde(rename = "type")]
    pub card_type: Option<String>,
    pub hashed_number: Option<String>,
}

/// Info en “processing_information”.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformationResponse {
    pub payment_solution: Option<String>,
    pub commerce_indicator: Option<String>,
    pub commerce_indicator_label: Option<String>,
    pub authorization_options: Option<AuthorizationOptions>,
    pub ecommerce_indicator: Option<String>,
}

/// AuthorizationOptions => “auth_type”, “initiator”, etc.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationOptions {
    pub auth_type: Option<String>,
    pub initiator: Option<Initiator>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Initiator {
    pub merchant_initiated_transaction: Option<MerchantInitiatedTransactionResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantInitiatedTransactionResponse {
    pub agreement_id: Option<String>,
    pub previous_transaction_id: Option<String>,
    pub original_authorized_amount: Option<String>,
    pub reason: Option<String>,
}

/// Token info => se puede tener `payment_instrument: Some(LafisePaymentInstrument)`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseTokenInformation {
    pub payment_instrument: Option<LafisePaymentInstrument>,
}

/// Info sobre “IssuerInformation” (banco emisor).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuerInformation {
    pub country: Option<enums::CountryAlpha2>,
    pub discretionary_data: Option<String>,
    pub country_specific_discretionary_data: Option<String>,
    pub response_code: Option<String>,
    pub pin_request_indicator: Option<String>,
}

/// CardResponseObject => suffix, prefix, expiration, type, etc.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardResponseObject {
    pub suffix: Option<String>,
    pub prefix: Option<String>,
    pub expiration_month: Option<Secret<String>>,
    pub expiration_year: Option<Secret<String>>,
    #[serde(rename = "type")]
    pub card_type: Option<String>,
}

/// Estructura de error principal en LAFISE (antes “BankOfAmericaErrorInformationResponse”).
/// En un solo lugar, con un único derive.
#[derive(Debug, Default, Deserialize, Clone, Serialize)]
pub struct LafiseErrorInformationResponse {
    pub id: String,
    pub error_information: LafiseErrorInformation,
}

/// Info concreta de error en LAFISE.
#[derive(Debug, Default, Deserialize, Clone, Serialize)]
pub struct LafiseErrorInformation {
    pub reason: Option<String>,
    pub message: Option<String>,
    pub details: Option<Vec<Details>>, // ← si tu 'Details' se clonará, este también debe tener Clone
}



// ========== Métodos para parsear la respuesta ========== //

impl<F, T> TryFrom<ResponseRouterData<F, LafiseSetupMandatesResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, LafiseSetupMandatesResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            LafiseSetupMandatesResponse::ClientReferenceInformation(info_response) => {
                let maybe_token_info = info_response.token_information.clone();
                let mandate_reference = maybe_token_info.and_then(|token_info| {
                    token_info.payment_instrument.map(|instr| MandateReference {
                        connector_mandate_id: Some(instr.id.expose()),
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    })
                });
                let mut mandate_status = map_lafise_attempt_status((info_response.status.clone(), false));
                // Para zero auth, si es “Authorized” => “Charged”.
                if matches!(mandate_status, enums::AttemptStatus::Authorized) {
                    mandate_status = enums::AttemptStatus::Charged;
                }
                let error_response = get_error_response_if_failure((&info_response, mandate_status, item.http_code));
                let connector_response = match item.data.payment_method {
                    common_enums::PaymentMethod::Card => info_response
                        .processor_information
                        .as_ref()
                        .and_then(|proc_info| {
                            info_response
                                .consumer_authentication_information
                                .as_ref()
                                .map(|auth_info| convert_to_additional_payment_method_connector_response(proc_info, auth_info))
                        })
                        .map(ConnectorResponseData::with_additional_payment_method_data),
                    _ => None,
                };

                Ok(Self {
                    status: mandate_status,
                    response: match error_response {
                        Some(error) => Err(error),
                        None => Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(info_response.id.clone()),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(mandate_reference),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: Some(info_response
                                .client_reference_information
                                .code
                                .clone()
                                .unwrap_or(info_response.id)),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                    },
                    connector_response,
                    ..item.data
                })
            }
            LafiseSetupMandatesResponse::ErrorInformation(error_response) => {
                let resp = Err(convert_to_error_response_from_error_info(&error_response, item.http_code));
                Ok(Self {
                    response: resp,
                    status: enums::AttemptStatus::Failure,
                    ..item.data
                })
            }
        }
    }
}

/// Aux: si la “status” indica error, devolvemos un ErrorResponse de conector.
fn get_error_response_if_failure(
    (info_response, status, http_code): (&LafiseClientReferenceResponse, enums::AttemptStatus, u16),
) -> Option<ErrorResponse> {
    if utils::is_payment_failure(status) {
        Some(get_error_response(
            &info_response.error_information,
            &info_response.risk_information,
            Some(status),
            http_code,
            info_response.id.clone(),
        ))
    } else {
        None
    }
}

/// Retorna PaymentResponseData (TransactionResponse) si no falló, o un Error si sí falló.
fn get_payment_response(
    (info_response, status, http_code): (&LafiseClientReferenceResponse, enums::AttemptStatus, u16),
) -> Result<PaymentsResponseData, ErrorResponse> {
    let error_response = get_error_response_if_failure((info_response, status, http_code));
    match error_response {
        Some(error) => Err(error),
        None => {
            let mandate_reference = info_response.token_information.clone().map(|token_info| {
                MandateReference {
                    connector_mandate_id: token_info
                        .payment_instrument
                        .map(|instr| instr.id.expose()),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                }
            });

            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(info_response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    info_response
                        .client_reference_information
                        .code
                        .clone()
                        .unwrap_or(info_response.id.clone()),
                ),
                incremental_authorization_allowed: None,
                charges: None,
            })
        }
    }
}

/// Estructura de “ClientProcessorInformation”, análoga a la original.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientProcessorInformation {
    avs: Option<Avs>,
    card_verification: Option<CardVerification>,
    processor: Option<ProcessorResponse>,
    network_transaction_id: Option<Secret<String>>,
    approval_code: Option<String>,
    merchant_advice: Option<MerchantAdvice>,
    response_code: Option<String>,
    ach_verification: Option<AchVerification>,
    system_trace_audit_number: Option<String>,
    event_status: Option<String>,
    retrieval_reference_number: Option<String>,
    consumer_authentication_response: Option<ConsumerAuthenticationResponse>,
    response_details: Option<String>,
    transaction_id: Option<Secret<String>>,
}

/// Estructuras de AVS, CardVerification, etc.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Avs {
    code: Option<String>,
    code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardVerification {
    result_code: Option<String>,
    result_code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorResponse {
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantAdvice {
    code: Option<String>,
    code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchVerification {
    result_code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumerAuthenticationResponse {
    code: Option<String>,
    code_raw: Option<String>,
}

/// Estructura RiskInformation: “rules”, “score”, etc.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRiskInformation {
    rules: Option<Vec<ClientRiskInformationRules>>,
    profile: Option<Profile>,
    score: Option<Score>,
    info_codes: Option<InfoCodes>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoCodes {
    address: Option<Vec<String>>,
    identity_change: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    factor_codes: Option<Vec<String>>,
    result: Option<RiskResult>,
    model_used: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RiskResult {
    StringVariant(String),
    IntVariant(u64),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    early_decision: Option<String>,
    name: Option<String>,
    decision: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientRiskInformationRules {
    name: Option<Secret<String>>,
}

/// “ProcessingInformationResponse”: ya se definió, se mantiene.

// ========== Implementaciones de TryFrom para Responses ========== //

impl<F>
    TryFrom<
        ResponseRouterData<F, LafisePaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            LafisePaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            LafisePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = map_lafise_attempt_status((
                    info_response.status.clone(),
                    item.data.request.is_auto_capture()?,
                ));
                let response = get_payment_response((&info_response, status, item.http_code));
                let connector_response = match item.data.payment_method {
                    common_enums::PaymentMethod::Card => info_response
                        .processor_information
                        .as_ref()
                        .and_then(|processor_information| {
                            info_response
                                .consumer_authentication_information
                                .as_ref()
                                .map(|consumer_auth_information| {
                                    convert_to_additional_payment_method_connector_response(
                                        processor_information,
                                        consumer_auth_information,
                                    )
                                })
                        })
                        .map(ConnectorResponseData::with_additional_payment_method_data),
                    _ => None,
                };
                Ok(Self {
                    status,
                    response,
                    connector_response,
                    ..item.data
                })
            }
            LafisePaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(map_error_response(&error_response.clone(), item, Some(enums::AttemptStatus::Failure)))
            }            
        }
    }
}

/// Construye un AdditionalPaymentMethodConnectorResponse::Card con info de AVS, ECI, etc.
fn convert_to_additional_payment_method_connector_response(
    processor_information: &ClientProcessorInformation,
    consumer_authentication_information: &ConsumerAuthenticationInformation,
) -> AdditionalPaymentMethodConnectorResponse {
    let payment_checks = Some(serde_json::json!({
        "avs_response": processor_information.avs,
        "card_verification": processor_information.card_verification,
        "approval_code": processor_information.approval_code,
        "consumer_authentication_response": processor_information.consumer_authentication_response,
        "cavv": consumer_authentication_information.cavv,
        "eci": consumer_authentication_information.eci,
        "eci_raw": consumer_authentication_information.eci_raw,
    }));

    let authentication_data = Some(serde_json::json!({
        "retrieval_reference_number": processor_information.retrieval_reference_number,
        "acs_transaction_id": consumer_authentication_information.acs_transaction_id,
        "system_trace_audit_number": processor_information.system_trace_audit_number,
    }));

    AdditionalPaymentMethodConnectorResponse::Card {
        authentication_data,
        payment_checks,
        card_network: None,
        domestic_network: None,
    }
}


impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            LafisePaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            LafisePaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            LafisePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = map_lafise_attempt_status((info_response.status.clone(), true));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            LafisePaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(map_error_response(&error_response.clone(), item, None))
            }
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            LafisePaymentsResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            LafisePaymentsResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            LafisePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = map_lafise_attempt_status((info_response.status.clone(), false));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            LafisePaymentsResponse::ErrorInformation(ref error_response)=> {
                Ok(map_error_response(&error_response.clone(), item, None))
            }
        }
    }
}

/// Estructura para la consulta de transacciones (PSync).
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseTransactionResponse {
    id: String,
    application_information: ApplicationInformation,
    client_reference_information: Option<ClientReferenceInformation>,
    processor_information: Option<ClientProcessorInformation>,
    processing_information: Option<ProcessingInformationResponse>,
    payment_information: Option<PaymentInformationResponse>,
    payment_insights_information: Option<PaymentInsightsInformation>,
    error_information: Option<LafiseErrorInformation>,
    fraud_marking_information: Option<FraudMarkingInformation>,
    risk_information: Option<ClientRiskInformation>,
    token_information: Option<LafiseTokenInformation>,
    reconciliation_id: Option<String>,
    consumer_authentication_information: Option<ConsumerAuthenticationInformation>,
}

/// Ejemplo de FraudMarkingInformation.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudMarkingInformation {
    reason: Option<String>,
}

/// ApplicationInformation: “status: Some(LafisePaymentStatus)”.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInformation {
    status: Option<LafisePaymentStatus>,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, LafiseTransactionResponse, PaymentsSyncData, PaymentsResponseData>,
    > for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, LafiseTransactionResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.application_information.status {
            Some(app_status) => {
                let status = map_lafise_attempt_status((app_status, item.data.request.is_auto_capture()?));
                let connector_response = match item.data.payment_method {
                    common_enums::PaymentMethod::Card => item
                        .response
                        .processor_information
                        .as_ref()
                        .and_then(|processor_information| {
                            item.response
                                .consumer_authentication_information
                                .as_ref()
                                .map(|consumer_auth_information| {
                                    convert_to_additional_payment_method_connector_response(
                                        processor_information,
                                        consumer_auth_information,
                                    )
                                })
                        })
                        .map(ConnectorResponseData::with_additional_payment_method_data),
                    _ => None,
                };

                // Checa si “status” => error => genera ErrorResponse
                if utils::is_payment_failure(status) {
                    Ok(Self {
                        response: Err(get_error_response(
                            &item.response.error_information,
                            &None,
                            Some(status),
                            item.http_code,
                            item.response.id.clone(),
                        )),
                        status: enums::AttemptStatus::Failure,
                        connector_response,
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: item
                                .response
                                .client_reference_information
                                .map(|cref| cref.code)
                                .unwrap_or(Some(item.response.id)),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        connector_response,
                        ..item.data
                    })
                }
            }
            None => Ok(Self {
                status: item.data.status,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.id),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
        }
    }
}

// ========== Manejo de Void, Capture, Refund ========== //

/// Estructura análoga a “BankOfAmericaCaptureRequest”.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseCaptureRequest {
    order_information: OrderInformation,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

/// Estructura “OrderInformation” con “amount_details”.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformation {
    amount_details: Amount,
}

impl TryFrom<&LafiseRouterData<&PaymentsCaptureRouterData>> for LafiseCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &LafiseRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let merchant_defined_information = value
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);
        Ok(Self {
            order_information: OrderInformation {
                amount_details: Amount {
                    total_amount: value.amount.to_owned(),
                    currency: value.router_data.request.currency,
                },
            },
            client_reference_information: ClientReferenceInformation {
                code: Some(value.router_data.connector_request_reference_id.clone()),
            },
            merchant_defined_information,
        })
    }
}

/// Petición a anulación (Void).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseVoidRequest {
    client_reference_information: ClientReferenceInformation,
    reversal_information: ReversalInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

/// Subcampos dentro de “reversal_information”.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReversalInformation {
    amount_details: Amount,
    reason: String,
}

impl TryFrom<&LafiseRouterData<&PaymentsCancelRouterData>> for LafiseVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: &LafiseRouterData<&PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_defined_information = value
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);

        Ok(Self {
            client_reference_information: ClientReferenceInformation {
                code: Some(value.router_data.connector_request_reference_id.clone()),
            },
            reversal_information: ReversalInformation {
                amount_details: Amount {
                    total_amount: value.amount.to_owned(),
                    currency: value.router_data.request.currency.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "Currency",
                        },
                    )?,
                },
                reason: value
                    .router_data
                    .request
                    .cancellation_reason
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "Cancellation Reason",
                    })?,
            },
            merchant_defined_information,
        })
    }
}

/// Estructura “RefundRequest”.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseRefundRequest {
    order_information: OrderInformation,
    client_reference_information: ClientReferenceInformation,
}

impl<F> TryFrom<&LafiseRouterData<&RefundsRouterData<F>>> for LafiseRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &LafiseRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            order_information: OrderInformation {
                amount_details: Amount {
                    total_amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                },
            },
            client_reference_information: ClientReferenceInformation {
                code: Some(item.router_data.request.refund_id.clone()),
            },
        })
    }
}

/// “RefundResponse”.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseRefundResponse {
    id: String,
    status: LafiseRefundStatus,
    error_information: Option<LafiseErrorInformation>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LafiseRefundStatus {
    Succeeded,
    Transmitted,
    Failed,
    Pending,
    Voided,
    Cancelled,
    #[serde(rename = "201")]
    TwoZeroOne,
}

/// Del RefundResponse => RefundStatus.
impl From<LafiseRefundResponse> for enums::RefundStatus {
    fn from(item: LafiseRefundResponse) -> Self {
        let error_reason = item
            .error_information
            .and_then(|error_info| error_info.reason);
        match item.status {
            LafiseRefundStatus::Succeeded | LafiseRefundStatus::Transmitted => Self::Success,
            LafiseRefundStatus::Cancelled
            | LafiseRefundStatus::Failed
            | LafiseRefundStatus::Voided => Self::Failure,
            LafiseRefundStatus::Pending => Self::Pending,
            LafiseRefundStatus::TwoZeroOne => {
                if error_reason == Some("PROCESSOR_DECLINED".to_string()) {
                    Self::Failure
                } else {
                    Self::Pending
                }
            }
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, LafiseRefundResponse>> for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, LafiseRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.clone());
        let response = if utils::is_refund_failure(refund_status) {
            Err(get_error_response(
                &item.response.error_information,
                &None,
                None,
                item.http_code,
                item.response.id,
            ))
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            })
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

/// Respuesta de RSync => “LafiseRsyncResponse”.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseRsyncResponse {
    id: String,
    application_information: Option<RsyncApplicationInformation>,
    error_information: Option<LafiseErrorInformation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RsyncApplicationInformation {
    status: Option<LafiseRefundStatus>,
}

impl TryFrom<RefundsResponseRouterData<RSync, LafiseRsyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, LafiseRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item
            .response
            .application_information
            .and_then(|application_information| application_information.status)
        {
            Some(status) => {
                let error_reason = item
                    .response
                    .error_information
                    .clone()
                    .and_then(|err_info| err_info.reason);
                let refund_status: enums::RefundStatus = match status {
                    LafiseRefundStatus::Succeeded | LafiseRefundStatus::Transmitted => {
                        enums::RefundStatus::Success
                    }
                    LafiseRefundStatus::Cancelled
                    | LafiseRefundStatus::Failed
                    | LafiseRefundStatus::Voided => enums::RefundStatus::Failure,
                    LafiseRefundStatus::Pending => enums::RefundStatus::Pending,
                    LafiseRefundStatus::TwoZeroOne => {
                        if error_reason == Some("PROCESSOR_DECLINED".to_string()) {
                            enums::RefundStatus::Failure
                        } else {
                            enums::RefundStatus::Pending
                        }
                    }
                };
                if utils::is_refund_failure(refund_status) {
                    if status == LafiseRefundStatus::Voided {
                        Err(get_error_response(
                            &Some(LafiseErrorInformation {
                                message: Some(constants::REFUND_VOIDED.to_string()),
                                reason: Some(constants::REFUND_VOIDED.to_string()),
                                details: None,
                            }),
                            &None,
                            None,
                            item.http_code,
                            item.response.id.clone(),
                        ))
                    } else {
                        Err(get_error_response(
                            &item.response.error_information,
                            &None,
                            None,
                            item.http_code,
                            item.response.id.clone(),
                        ))
                    }
                } else {
                    Ok(RefundsResponseData {
                        connector_refund_id: item.response.id.clone(),
                        refund_status,
                    })
                }
            }
            None => Ok(RefundsResponseData {
                connector_refund_id: item.response.id.clone(),
                refund_status: match item.data.response {
                    Ok(ref res) => res.refund_status,
                    Err(_) => enums::RefundStatus::Pending,
                },
            }),
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

// ========== Mensajes de Error Genéricos (400 / 500) ========== //

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseStandardErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LafiseServerErrorResponse {
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<Reason>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Reason {
    SystemError,
    ServerTimeout,
    ServiceTimeout,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LafiseAuthenticationErrorResponse {
    pub response: AuthenticationErrorInformation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LafiseErrorResponse {
    AuthenticationError(LafiseAuthenticationErrorResponse),
    StandardError(LafiseStandardErrorResponse),
}

/// Info de error en campo “details”.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ErrorInformation {
    pub message: String,
    pub reason: String,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AuthenticationErrorInformation {
    pub rmsg: String,
}

/// Aux: genera un “ErrorResponse” a partir de error_info.
fn get_error_response(
    error_data: &Option<LafiseErrorInformation>,
    risk_information: &Option<ClientRiskInformation>,
    attempt_status: Option<enums::AttemptStatus>,
    status_code: u16,
    transaction_id: String,
) -> ErrorResponse {
    let avs_message = risk_information.clone().map(|info| {
        info.rules.map(|rules| {
            rules
                .iter()
                .map(|r| {
                    r.name.clone().map_or("".to_string(), |n| {
                        format!(" , {}", n.clone().expose())
                    })
                })
                .collect::<Vec<String>>()
                .join("")
        })
    }).unwrap_or(Some("".to_string()));

    let detailed_error_info = error_data.as_ref().and_then(|err| {
        err.details.as_ref().map(|error_details| {
            error_details
                .iter()
                .map(|d| format!("{} : {}", d.field, d.reason))
                .collect::<Vec<_>>()
                .join(", ")
        })
    });

    let reason = get_error_reason(
        error_data.clone().and_then(|e| e.message),
        detailed_error_info,
        avs_message,
    );
    let error_message = error_data.clone().and_then(|e| e.reason);

    ErrorResponse {
        code: error_message
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
        message: error_message
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
        reason,
        status_code,
        attempt_status,
        connector_transaction_id: Some(transaction_id.clone()),
        network_advice_code: None,
network_decline_code: None,
network_error_message: None,

    }
}

/// Crea un RouterData con error y su status en “Failure”.
fn map_error_response<F, T>(
    error_response: &LafiseErrorInformationResponse,
    item: ResponseRouterData<F, LafisePaymentsResponse, T, PaymentsResponseData>,
    transaction_status: Option<enums::AttemptStatus>,
) -> RouterData<F, T, PaymentsResponseData> {
    let detailed_error_info = error_response
        .error_information
        .details
        .as_ref()
        .map(|details| {
            details
                .iter()
                .map(|d| format!("{} : {}", d.field, d.reason))
                .collect::<Vec<_>>()
                .join(", ")
        });
    let reason = get_error_reason(
        error_response.error_information.message.clone(),
        detailed_error_info,
        None,
    );
    let error_message = error_response.error_information.reason.clone();

    let response = Err(ErrorResponse {
        code: error_message
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
        message: error_message
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
        reason,
        status_code: item.http_code,
        attempt_status: None,
        connector_transaction_id: Some(error_response.id.clone()),
        network_advice_code: None,
network_decline_code: None,
network_error_message: None,

    });

    match transaction_status {
        Some(status) => RouterData {
            response,
            status,
            ..item.data
        },
        None => RouterData {
            response,
            ..item.data
        },
    }
}

/// Crea un “ErrorResponse” a partir de “LafiseErrorInformationResponse + status code”.
fn convert_to_error_response_from_error_info(
    error_response: &LafiseErrorInformationResponse,
    status_code: u16,
) -> ErrorResponse {
    let detailed_error_info = error_response
        .error_information
        .to_owned()
        .details
        .map(|err_details| {
            err_details
                .iter()
                .map(|d| format!("{} : {}", d.field, d.reason))
                .collect::<Vec<_>>()
                .join(", ")
        });

    let reason = get_error_reason(
        error_response.error_information.message.clone(),
        detailed_error_info,
        None,
    );
    ErrorResponse {
        code: error_response
            .error_information
            .reason
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
        message: error_response
            .error_information
            .reason
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
        reason,
        status_code,
        attempt_status: None,
        connector_transaction_id: Some(error_response.id.clone()),
        network_advice_code: None,
network_decline_code: None,
network_error_message: None,

    }
}

/// Función que combina `error_info` + `detailed_error_info` + `avs_error_info`.
pub fn get_error_reason(
    error_info: Option<String>,
    detailed_error_info: Option<String>,
    avs_error_info: Option<String>,
) -> Option<String> {
    match (error_info, detailed_error_info, avs_error_info) {
        (Some(message), Some(details), Some(avs_msg)) => {
            Some(format!("{}, detailed_error_information: {}, avs_message: {}", message, details, avs_msg))
        }
        (Some(message), Some(details), None) => {
            Some(format!("{}, detailed_error_information: {}", message, details))
        }
        (Some(message), None, Some(avs_msg)) => {
            Some(format!("{}, avs_message: {}", message, avs_msg))
        }
        (None, Some(details), Some(avs_msg)) => {
            Some(format!("{}, avs_message: {}", details, avs_msg))
        }
        (Some(message), None, None) => Some(message),
        (None, Some(details), None) => Some(details),
        (None, None, Some(avs_msg)) => Some(avs_msg),
        (None, None, None) => None,
    }
}