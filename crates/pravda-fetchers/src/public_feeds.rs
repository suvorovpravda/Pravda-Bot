//! Auth-free public market and news connectors. Never submit orders or load API keys.
//! All network addresses are pinned HTTPS; no caller-provided URL can be requested.
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

const FX_URL: &str = "https://api.frankfurter.dev/v2/providers/ecb/rates?quotes=USD,CHF,GBP,JPY";
const NEWS_URL: &str = "https://api.gdeltproject.org/api/v2/doc/doc?query=%28natural%20gas%20OR%20energy%20security%20OR%20geopolitical%20risk%29&mode=artlist&format=json&maxrecords=12&timespan=24h&sort=datedesc";
const ENTSOG_URL: &str = "https://transparency.entsog.eu/api/v1/operationalData?indicator=Physical%20Flow&periodType=day&limit=50";

async fn public_json(endpoint: &'static str) -> Result<Value, &'static str> {
    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "NETWORK_UNAVAILABLE")?;
    let mut response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|_| "NETWORK_UNAVAILABLE")?;
    if !response.status().is_success() {
        return Err("SOURCE_HTTP_ERROR");
    }
    if response.content_length().is_some_and(|n| n > 1_048_576) {
        return Err("SOURCE_TOO_LARGE");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "NETWORK_UNAVAILABLE")? {
        if bytes.len().saturating_add(chunk.len()) > 1_048_576 {
            return Err("SOURCE_TOO_LARGE");
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "SOURCE_INVALID_JSON")
}

#[derive(Deserialize)]
struct FxRate {
    date: String,
    base: String,
    quote: String,
    rate: f64,
}

fn fx_from_json(value: Value) -> Result<Value, &'static str> {
    let rows: Vec<FxRate> = serde_json::from_value(value).map_err(|_| "INVALID_FX_SCHEMA")?;
    let mut rates = serde_json::Map::new();
    let mut date: Option<String> = None;
    for row in rows {
        if row.base != "EUR"
            || !matches!(row.quote.as_str(), "USD" | "CHF" | "GBP" | "JPY")
            || !row.rate.is_finite()
            || row.rate <= 0.0
            || row.date.len() != 10
        {
            continue;
        }
        if let Some(previous) = &date {
            if previous != &row.date {
                return Err("MIXED_FX_DATES");
            }
        } else {
            date = Some(row.date.clone());
        }
        rates.insert(row.quote, json!(row.rate));
    }
    if !rates.contains_key("USD") || !rates.contains_key("CHF") {
        return Err("MISSING_FX_PAIRS");
    }
    Ok(json!({
        "status":"delayed", "freshness":"delayed",
        "source":"BCE / Frankfurter (ECB reference)",
        "source_url":"https://frankfurter.dev/",
        "date":date, "rates":rates, "timestamp":null,
        "notice":"Reference quotidienne ECB. Pas un prix d'exécution."
    }))
}

/// Reference rates are never represented as a tick-by-tick Forex feed.
pub async fn reference_fx() -> Value {
    match public_json(FX_URL).await.and_then(fx_from_json) {
        Ok(value) => value,
        Err(reason) => json!({"status":"unavailable","rates":{},"timestamp":null,
            "source":"BCE / Frankfurter","notice":reason}),
    }
}

fn articles_from_json(value: Value) -> Result<Value, &'static str> {
    let source = value
        .get("articles")
        .and_then(Value::as_array)
        .ok_or("INVALID_NEWS_SCHEMA")?;
    let mut articles = Vec::new();
    for row in source.iter().take(30) {
        let (Some(title), Some(link)) = (
            row.get("title").and_then(Value::as_str),
            row.get("url").and_then(Value::as_str),
        ) else {
            continue;
        };
        if title.trim().is_empty() || title.len() > 300 || link.len() > 1000 {
            continue;
        }
        let Ok(url) = Url::parse(link) else {
            continue;
        };
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            continue;
        }
        articles.push(json!({"title":title,"url":link,
            "source":row.get("domain").and_then(Value::as_str).unwrap_or("GDELT"),
            "published_at":row.get("seendate").and_then(Value::as_str).unwrap_or("")}));
        if articles.len() >= 12 {
            break;
        }
    }
    if articles.is_empty() {
        return Err("NO_VERIFIED_ARTICLES");
    }
    Ok(json!({"status":"delayed","source":"GDELT DOC 2.0",
        "source_url":"https://www.gdeltproject.org/",
        "retrieved_at":chrono::Utc::now().to_rfc3339(),
        "articles":articles,"notice":"Couverture médiatique, pas un événement confirmé."}))
}

pub async fn geopolitical_news() -> Value {
    match public_json(NEWS_URL).await.and_then(articles_from_json) {
        Ok(value) => value,
        Err(reason) => json!({"status":"unavailable","articles":[],
            "source":"GDELT DOC 2.0","notice":reason,"retrieved_at":null}),
    }
}

fn entsog_from_json(value: Value) -> Result<Value, &'static str> {
    let rows = value
        .get("operationalData")
        .or_else(|| value.get("operationaldata"))
        .and_then(Value::as_array)
        .ok_or("INVALID_ENTSOG_SCHEMA")?;
    let mut observations = Vec::new();
    for row in rows.iter().take(100) {
        let point = row.get("pointLabel").and_then(Value::as_str).unwrap_or("");
        let direction = row
            .get("directionKey")
            .and_then(Value::as_str)
            .unwrap_or("");
        let amount = row.get("value").and_then(Value::as_f64).or_else(|| {
            row.get("value")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<f64>().ok())
        });
        let Some(value) = amount else { continue };
        if point.is_empty() || point.len() > 150 || !value.is_finite() || value < 0.0 {
            continue;
        }
        let dir = if direction.to_ascii_lowercase().contains("entry") {
            "entry"
        } else if direction.to_ascii_lowercase().contains("exit") {
            "exit"
        } else {
            continue;
        };
        observations.push(json!({"point":point,"direction":dir,"value":value,
            "unit":row.get("unit").and_then(Value::as_str).unwrap_or("kWh/d"),
            "operator":row.get("operatorLabel").and_then(Value::as_str).unwrap_or(""),
            "period_from":row.get("periodFrom").and_then(Value::as_str).unwrap_or(""),
            "updated_at":row.get("lastUpdateDate").and_then(Value::as_str).unwrap_or(""),
            "country":row.get("countryLabel").and_then(Value::as_str).unwrap_or("")}));
        if observations.len() >= 30 {
            break;
        }
    }
    if observations.is_empty() {
        return Err("NO_VERIFIED_ENTSOG_OBSERVATIONS");
    }
    let entries = observations
        .iter()
        .filter(|x| x["direction"] == "entry")
        .count();
    let exits = observations.len() - entries;
    Ok(
        json!({"status":"delayed","source":"ENTSOG Transparency Platform",
        "source_url":"https://transparency.entsog.eu/","observations":observations,
        "entry_count":entries,"exit_count":exits,"latest_source_update":null,
        "notice":"Échantillon limité de points publiés ; pas de total pays."}),
    )
}

pub async fn energy_flows() -> Value {
    match public_json(ENTSOG_URL).await.and_then(entsog_from_json) {
        Ok(value) => value,
        Err(reason) => json!({"status":"unavailable","source":"ENTSOG",
            "observations":[],"entry_count":null,"exit_count":null,
            "latest_source_update":null,"notice":reason}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fx_rejects_nans_and_missing_pairs() {
        assert!(
            fx_from_json(json!([{"date":"2026-11-02","base":"EUR","quote":"USD","rate":1.0}]))
                .is_err()
        );
        let result = fx_from_json(json!([
            {"date":"2026-11-02","base":"EUR","quote":"USD","rate":1.12},
            {"date":"2026-11-02","base":"EUR","quote":"CHF","rate":0.92}]))
        .unwrap();
        assert_eq!(result["rates"]["CHF"], 0.92);
    }
    #[test]
    fn gdelt_rejects_scripts_and_http_links() {
        assert!(articles_from_json(
            json!({"articles":[{"title":"Bad","url":"javascript:alert(1)"}]})
        )
        .is_err());
        let out=articles_from_json(json!({"articles":[{"title":"Public energy news","url":"https://example.org/news","domain":"example.org"}]})).unwrap();
        assert_eq!(out["articles"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn entsog_does_not_fabricate_observations() {
        assert!(entsog_from_json(json!({"operationalData":[]})).is_err());
    }
}
