use thiserror::Error;

/// Optional external feeds are deliberately disabled until a verified source
/// contract and integration tests exist. Never return fictitious empty datasets.
#[derive(Debug, Error)]
pub enum EnergyError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parse error")]
    Parse,
    #[error("data source unavailable: {0}")]
    SourceUnavailable(&'static str),
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

/// No ENTSOG integration currently exists. Fail closed; callers must not
/// interpret a successful empty result as genuine zero flows.
#[deprecated(note = "Fonctionnalité en attente de source de données fiable")]
pub async fn get_entsog_flows() -> Result<EntsogResult, EnergyError> {
    Err(EnergyError::SourceUnavailable("ENTSOG"))
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

/// No GDELT integration currently exists; do not fabricate a retrieval time
/// or return a successful empty news set.
#[deprecated(note = "Fonctionnalité en attente de source de données fiable")]
pub async fn get_geopolitical_news() -> Result<NewsResult, EnergyError> {
    Err(EnergyError::SourceUnavailable("GDELT"))
}

/// No authenticated, supported TTF price source is configured.
#[deprecated(note = "Fonctionnalité en attente de source de données fiable")]
pub fn get_ttf_price_status() -> Result<serde_json::Value, EnergyError> {
    Err(EnergyError::SourceUnavailable("TTF"))
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unavailable_energy_feeds_cannot_look_like_success() {
        assert!(matches!(
            get_entsog_flows().await,
            Err(EnergyError::SourceUnavailable("ENTSOG"))
        ));
        assert!(matches!(
            get_geopolitical_news().await,
            Err(EnergyError::SourceUnavailable("GDELT"))
        ));
        assert!(matches!(
            get_ttf_price_status(),
            Err(EnergyError::SourceUnavailable("TTF"))
        ));
    }
}
