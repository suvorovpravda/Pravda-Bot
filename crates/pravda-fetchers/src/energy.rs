use chrono::Utc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnergyError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parse error")]
    Parse,
}

#[derive(Debug)]
pub struct EntsogResult {
    pub status: String,
    pub source: String,
    pub source_url: String,
    pub retrieved_at: String,
    pub latest_source_update: Option<String>,
    pub entry_count: usize,
    pub exit_count: usize,
    pub notice: String,
    pub observations: Vec<serde_json::Value>,
}

pub async fn get_entsog_flows() -> Result<EntsogResult, EnergyError> {
    // Simplified stub for now - real implementation would call ENTSOG API
    Ok(EntsogResult {
        status: "unavailable".to_string(),
        source: "ENTSOG".to_string(),
        source_url: "https://transparency.entsog.eu/".to_string(),
        retrieved_at: Utc::now().to_rfc3339(),
        latest_source_update: None,
        entry_count: 0,
        exit_count: 0,
        notice: "Stub implementation".to_string(),
        observations: vec![],
    })
}

#[derive(Debug)]
pub struct NewsResult {
    pub status: String,
    pub source: String,
    pub source_url: String,
    pub retrieved_at: String,
    pub notice: String,
    pub articles: Vec<serde_json::Value>,
}

pub async fn get_geopolitical_news() -> Result<NewsResult, EnergyError> {
    Ok(NewsResult {
        status: "unavailable".to_string(),
        source: "GDELT".to_string(),
        source_url: "https://www.gdeltproject.org/".to_string(),
        retrieved_at: Utc::now().to_rfc3339(),
        notice: "Stub implementation".to_string(),
        articles: vec![],
    })
}

pub fn get_ttf_price_status() -> serde_json::Value {
    serde_json::json!({
        "status": "unavailable",
        "benchmark": "Dutch TTF Natural Gas Futures",
        "value": null,
        "unit": "EUR/MWh",
        "source_url": "https://www.ice.com/products/27996665/Dutch-TTF-Natural-Gas-Futures",
        "notice": "No TTF price feed configured"
    })
}
