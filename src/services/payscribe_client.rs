use reqwest::StatusCode;
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct PayscribeClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl PayscribeClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            http: reqwest::Client::new(),
        }
    }

    fn auth(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, String> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let res = self
            .http
            .get(url)
            .header("Authorization", self.auth())
            .header("Content-Type", "application/json")
            .query(query)
            .send()
            .await
            .map_err(|e| format!("Payscribe network error: {e}"))?;

        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Payscribe error {}: {}", status.as_u16(), text));
        }

        serde_json::from_str::<T>(&text)
            .map_err(|e| format!("Payscribe parse error: {e}. Raw: {text}"))
    }

    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let res = self
            .http
            .post(url)
            .header("Authorization", self.auth())
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Payscribe network error: {e}"))?;

        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        // Payscribe can return 201 pending - still "not failure".
        if status != StatusCode::OK && status != StatusCode::CREATED {
            return Err(format!("Payscribe error {}: {}", status.as_u16(), text));
        }

        serde_json::from_str::<T>(&text)
            .map_err(|e| format!("Payscribe parse error: {e}. Raw: {text}"))
    }
}
