#![allow(dead_code)]

// FILE: src/payscribe_client/client.rs

use reqwest::{header, Client, Response};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use tracing::debug;

use crate::bills_service::models::BillService;

/* ─────────────────────────────
   Shared Response Envelope
   Payscribe usually returns:
   { status: bool, description: "...", message: <any>, status_code: <int> }
───────────────────────────── */

#[derive(Debug, Deserialize)]
struct PayscribeEnvelope {
    status: bool,
    description: Option<String>,
    message: Option<Value>,
    #[serde(default)]
    status_code: Option<u16>,
}

/* ─────────────────────────────
   Validation Responses (normalized)
───────────────────────────── */

#[derive(Debug, Deserialize, Clone)]
pub struct PayscribeValidationData {
    pub customer_name: String,
    pub customer_address: Option<String>,
    #[serde(rename = "productCode")]
    pub product_code: String,
    #[serde(rename = "productToken")]
    pub product_token: String,
}

/* ─────────────────────────────
   Vend Responses (normalized)
───────────────────────────── */

#[derive(Debug, Deserialize, Clone)]
pub struct PayscribeVendData {
    pub status: Option<String>,
    #[serde(rename = "trans_id")]
    pub provider_reference: String,
}

/* ─────────────────────────────
   Client
───────────────────────────── */

#[derive(Clone)]
pub struct PayscribeClient {
    http_client: Client,
    base_url: String,
}

impl PayscribeClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                .expect("Invalid Payscribe API key"),
        );
        headers.insert(
            "Content-Type",
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Accept",
            header::HeaderValue::from_static("application/json"),
        );

        let http_client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build Payscribe HTTP client");

        Self {
            http_client,
            base_url,
        }
    }

    fn url(&self, path: &str) -> String {
        let b = self.base_url.trim_end_matches('/');
        let p = path.trim_start_matches('/');
        format!("{}/{}", b, p)
    }

    /* ─────────────────────────────
       Services (NO MOCK)
       These must call Payscribe endpoints.
       If Payscribe does not provide them, do DB-backed services instead.
    ───────────────────────────── */

    /// Used by: GET /bills/services?category=airtime
    ///
    /// Expected endpoint (adjust if your Payscribe differs):
    /// GET {base_url}/airtime/networks
    pub async fn get_airtime_networks(&self) -> Result<Vec<BillService>, String> {
        let url = self.url("/airtime/networks");
        debug!(%url, "Fetching airtime networks (services)");

        let res = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        // We accept either:
        // message: [{code, provider, label, service_type, min_amount, max_amount}, ...]
        // OR message.details: [...]
        let v = self.parse_enveloped_value(res).await?;
        self.extract_bill_services(&v)
    }

    /// Used by: GET /bills/services?category=data&network=mtn
    ///
    /// You already had: GET /data/plans?network=mtn
    /// This converts result into Vec<BillService> if possible.
    pub async fn get_data_plans(&self, network: &str) -> Result<Vec<BillService>, String> {
        let url = self.url("/data/plans");
        debug!(%url, %network, "Fetching data plans (services)");

        let res = self
            .http_client
            .get(&url)
            .query(&[("network", network)])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let v = self.parse_enveloped_value(res).await?;
        self.extract_bill_services(&v)
    }

    /// Used by: GET /bills/services?category=electricity
    ///
    /// Expected endpoint (adjust if your Payscribe differs):
    /// GET {base_url}/electricity/services
    pub async fn get_electricity_services(&self) -> Result<Vec<BillService>, String> {
        let url = self.url("/electricity/services");
        debug!(%url, "Fetching electricity services");

        let res = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let v = self.parse_enveloped_value(res).await?;
        self.extract_bill_services(&v)
    }

    /// Used by: GET /bills/services?category=cable_tv
    ///
    /// Expected endpoint (adjust if your Payscribe differs):
    /// GET {base_url}/cable/services
    pub async fn get_cable_services(&self) -> Result<Vec<BillService>, String> {
        let url = self.url("/cable/services");
        debug!(%url, "Fetching cable TV services");

        let res = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let v = self.parse_enveloped_value(res).await?;
        self.extract_bill_services(&v)
    }

    /* ─────────────────────────────
       Airtime
    ───────────────────────────── */

    /// POST {base_url}/airtime_to_wallet/vend
    pub async fn vend_airtime(
        &self,
        network: &str,
        amount: i64,
        phone_number: &str,
        from: &str,
        reference: &str,
    ) -> Result<PayscribeVendData, String> {
        let url = self.url("/airtime_to_wallet/vend");

        let payload = json!({
            "network": network,
            "amount": amount.to_string(),
            "phone_number": phone_number,
            "from": from,
            "ref": reference
        });

        debug!(%url, %reference, "Sending airtime vend request");

        let res = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_vend_details(res).await
    }

    /* ─────────────────────────────
       Data Bundles
    ───────────────────────────── */

    /// POST {base_url}/data/vend
    ///
    /// IMPORTANT:
    /// - We pass our `reference` as "ref" for reconciliation.
    pub async fn vend_data(
        &self,
        plan_code: &str,
        phone: &str,
        reference: &str,
    ) -> Result<PayscribeVendData, String> {
        let url = self.url("/data/vend");

        let payload = json!({
            "plan": plan_code,
            "phone": phone,
            "ref": reference
        });

        debug!(%url, %reference, "Sending data vend request");

        let res = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_vend_details(res).await
    }

    /* ─────────────────────────────
       Electricity (Validate + Vend)
    ───────────────────────────── */

    /// RAW validate call.
    /// Used by handler: validate_electricity_raw(...) then parse_validation(&raw)
    pub async fn validate_electricity_raw(
        &self,
        meter_number: &str,
        meter_type: &str,
        service: &str,
        amount: i64,
    ) -> Result<Value, String> {
        let url = self.url("/electricity/validate");

        let payload = json!({
            "meter_number": meter_number,
            "meter_type": meter_type,
            "service": service,
            "amount": amount
        });

        debug!(%url, %meter_number, "Validating electricity (raw)");

        let res = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_enveloped_value(res).await
    }

    /// Optional convenience method if you want direct typed call.
    pub async fn validate_electricity(
        &self,
        meter_number: &str,
        meter_type: &str,
        service: &str,
        amount: i64,
    ) -> Result<PayscribeValidationData, String> {
        let raw = self
            .validate_electricity_raw(meter_number, meter_type, service, amount)
            .await?;
        self.parse_validation(&raw)
    }

    /// POST {base_url}/electricity/vend
    pub async fn vend_electricity(
        &self,
        product_code: &str,
        product_token: &str,
        reference: &str,
    ) -> Result<PayscribeVendData, String> {
        let url = self.url("/electricity/vend");

        let payload = json!({
            "productCode": product_code,
            "productToken": product_token,
            "ref": reference
        });

        debug!(%url, %reference, "Vending electricity");

        let res = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_vend_details(res).await
    }

    /* ─────────────────────────────
       Cable TV (Bouquets + Validate + Vend)
    ───────────────────────────── */

    /// GET {base_url}/cable/bouquets?service=dstv
    ///
    /// NOTE: Your previous code used "/bouquets". I aligned to your router:
    /// GET /bills/cable/bouquets -> client should call /cable/bouquets
    pub async fn get_cable_bouquets(&self, service: &str) -> Result<Value, String> {
        let url = self.url("/cable/bouquets");
        debug!(%url, %service, "Fetching cable bouquets");

        let res = self
            .http_client
            .get(&url)
            .query(&[("service", service)])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_enveloped_value(res).await
    }

    /// RAW validate call.
    /// Used by handler: validate_cable_tv_raw(...) then parse_validation(&raw)
    pub async fn validate_cable_tv_raw(
        &self,
        service: &str,
        account_number: &str,
    ) -> Result<Value, String> {
        let url = self.url("/cable/validate");

        let payload = json!({
            "service": service,
            "account_number": account_number
        });

        debug!(%url, %account_number, "Validating cable TV (raw)");

        let res = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_enveloped_value(res).await
    }

    pub async fn validate_cable_tv(
        &self,
        service: &str,
        account_number: &str,
    ) -> Result<PayscribeValidationData, String> {
        let raw = self.validate_cable_tv_raw(service, account_number).await?;
        self.parse_validation(&raw)
    }

    /// POST {base_url}/cable/vend
    pub async fn vend_cable_tv(
        &self,
        product_code: &str,
        product_token: &str,
        reference: &str,
    ) -> Result<PayscribeVendData, String> {
        let url = self.url("/cable/vend");

        let payload = json!({
            "productCode": product_code,
            "productToken": product_token,
            "ref": reference
        });

        debug!(%url, %reference, "Vending cable TV");

        let res = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_vend_details(res).await
    }

    /* ─────────────────────────────
       ePins (List + Vend)
    ───────────────────────────── */

    /// GET {base_url}/epins
    pub async fn get_epins(&self) -> Result<Value, String> {
        let url = self.url("/epins");
        debug!(%url, "Fetching epins");

        let res = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_enveloped_value(res).await
    }

    /// POST {base_url}/epins/vend
    pub async fn vend_epin(&self, payload: &Value) -> Result<Value, String> {
        let url = self.url("/epins/vend");
        debug!(%url, "Vending epin");

        let res = self
            .http_client
            .post(&url)
            .json(payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        self.parse_enveloped_value(res).await
    }

    /* ─────────────────────────────
       Normalizers used by handlers
    ───────────────────────────── */

    /// Handler expects: parse_validation(&raw) -> PayscribeValidationData
    ///
    /// Accepts different shapes:
    /// - message: { customer_name, customer_address, productCode, productToken }
    /// - message.details: { ...same fields... }
    pub fn parse_validation(&self, raw: &Value) -> Result<PayscribeValidationData, String> {
        // raw is already the "message" extracted by parse_enveloped_value()
        let candidate = if raw.get("customer_name").is_some() && raw.get("productCode").is_some() {
            raw.clone()
        } else if let Some(d) = raw.get("details") {
            d.clone()
        } else {
            raw.clone()
        };

        serde_json::from_value(candidate).map_err(|e| format!("Failed to parse validation: {}", e))
    }

    /* ─────────────────────────────
       Internal parsing helpers
    ───────────────────────────── */

    async fn parse_enveloped_value(&self, res: Response) -> Result<Value, String> {
        let status = res.status();
        let body = res.text().await.map_err(|e| e.to_string())?;

        // HTTP-level error
        if !status.is_success() {
            return Err(format!("Payscribe HTTP error: {} - {}", status, body));
        }

        let env: PayscribeEnvelope =
            serde_json::from_str(&body).map_err(|e| format!("Invalid JSON: {} - {}", e, body))?;

        if !env.status {
            let msg = env
                .description
                .clone()
                .or_else(|| {
                    env.message
                        .as_ref()
                        .and_then(|m| m.as_str().map(|s| s.to_string()))
                })
                .unwrap_or_else(|| "Payscribe returned failure".to_string());

            return Err(format!(
                "Payscribe failure: {} (status_code={:?})",
                msg, env.status_code
            ));
        }

        env.message
            .ok_or_else(|| "Payscribe response missing `message`".to_string())
    }

    /// For vend endpoints where we expect `message.details` to include { trans_id, status }
    async fn parse_vend_details(&self, res: Response) -> Result<PayscribeVendData, String> {
        let v = self.parse_enveloped_value(res).await?;

        let details = v.get("details").cloned().unwrap_or(v); // sometimes provider returns details directly

        serde_json::from_value(details).map_err(|e| format!("Failed to parse vend details: {}", e))
    }

    /// Extract bill services from provider response.
    /// Accepts:
    /// - message: [ ...BillService... ]
    /// - message.details: [ ...BillService... ]
    fn extract_bill_services(&self, v: &Value) -> Result<Vec<BillService>, String> {
        let arr = if v.is_array() {
            v.clone()
        } else if let Some(d) = v.get("details") {
            d.clone()
        } else if let Some(d) = v.get("data") {
            d.clone()
        } else {
            v.clone()
        };

        serde_json::from_value(arr).map_err(|e| format!("Failed to parse services list: {}", e))
    }

    #[allow(dead_code)]
    async fn parse_enveloped<T: DeserializeOwned>(&self, res: Response) -> Result<T, String> {
        let v = self.parse_enveloped_value(res).await?;
        serde_json::from_value(v).map_err(|e| e.to_string())
    }
}
