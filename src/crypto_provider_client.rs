use serde::Serialize;

#[derive(Serialize)]
pub struct CryptoAddress {
    pub address: String,
    pub memo_tag: Option<String>,
}

#[derive(Clone)]
pub struct CryptoProviderClient;

impl CryptoProviderClient {
    pub fn new() -> Self {
        Self {}
    }

    /// Mock: Generate a new deposit address
    pub async fn get_deposit_address(
        &self,
        asset: &str,
        network: &str,
    ) -> Result<CryptoAddress, String> {
        // In a real app, this would be an API call
        Ok(CryptoAddress {
            address: format!("MOCK_{}_{}_{}", asset, network, uuid::Uuid::new_v4()),
            memo_tag: if network == "BEP20" { Some("12345".to_string()) } else { None },
        })
    }
    
    /// Mock: Get a conversion quote
    pub async fn get_quote(&self, from: &str, to: &str) -> Result<f64, String> {
        if from == "USD" && to == "NGN" {
            Ok(1495.50) // Mock rate
        } else {
            Err("Invalid pair".to_string())
        }
    }

    /// Mock: Send crypto (withdraw)
    pub async fn send_crypto(
        &self,
        asset: &str,
        network: &str,
        amount: f64,
        to_address: &str,
    ) -> Result<String, String> {
        // In a real app, this would be an API call
        println!(
            "MOCK_PROVIDER: Sending {} {} on {} to {}",
            amount, asset, network, to_address
        );
        Ok(format!("mock_tx_{}", uuid::Uuid::new_v4()))
    }
}