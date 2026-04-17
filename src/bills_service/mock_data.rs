// This file mocks a bill payment provider's API
use super::models::{BillProvider, BillProduct, FormFieldSpec};
use serde_json::json;

// Mock providers
#[allow(dead_code)]
pub fn get_mock_providers(service: &str) -> Vec<BillProvider> {
    match service {
        "cable" => vec![
            BillProvider { code: "dstv", name: "DStv" },
            BillProvider { code: "gotv", name: "GOtv" },
            BillProvider { code: "startimes", name: "StarTimes" },
        ],
        "electricity" => vec![
            BillProvider { code: "ikedc", name: "Ikeja Electric" },
            BillProvider { code: "ekedc", name: "Eko Electric" },
            BillProvider { code: "aedc", name: "Abuja Electric" },
        ],
        _ => vec![],
    }
}

// Mock products (e.g., DStv bouquets)
#[allow(dead_code)]
pub fn get_mock_products(provider: &str) -> Vec<BillProduct> {
    if provider == "dstv" {
        vec![
            BillProduct { code: "dstv-padi", name: "DStv Padi", price: 250000 },
            BillProduct { code: "dstv-yanga", name: "DStv Yanga", price: 400000 },
            BillProduct { code: "dstv-premium", name: "DStv Premium", price: 2100000 },
        ]
    } else {
        vec![]
    }
}

// Mock form fields
#[allow(dead_code)]
pub fn get_mock_schema(service: &str, _provider: &str) -> Vec<FormFieldSpec> {
    match service {
        "cable" => vec![
            FormFieldSpec {
                key: "smartcard".to_string(),
                label: "Smartcard Number".to_string(),
                required: true,
                field_type: "text".to_string(),
                options: None,
            },
        ],
        "electricity" => vec![
            FormFieldSpec {
                key: "meter_number".to_string(),
                label: "Meter Number".to_string(),
                required: true,
                field_type: "text".to_string(),
                options: None,
            },
            FormFieldSpec {
                key: "meter_type".to_string(),
                label: "Meter Type".to_string(),
                required: true,
                field_type: "select".to_string(),
                options: Some(vec![json!({"code": "prepaid", "name": "Prepaid"}), json!({"code": "postpaid", "name": "Postpaid"})]),
            },
        ],
        _ => vec![],
    }
}
