use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ApiResponse<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct DailyReadiness {
    pub day: String,
    pub score: u32,
    pub contributors: ReadinessContributors,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ReadinessContributors {
    pub hrv_balance: Option<u32>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct DailySleep {
    pub day: String,
    pub score: u32,
}

pub struct OuraClient {
    client: Client,
    token: String,
}

impl OuraClient {
    pub fn new() -> Result<Self> {
        let token = std::env::var("OURA_TOKEN").context("OURA_TOKEN not set")?;
        Ok(Self {
            client: Client::new(),
            token,
        })
    }

    fn next_day(date: &str) -> Result<String> {
        let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").context("Invalid date format")?;
        Ok(d.succ_opt()
            .expect("date overflow")
            .format("%Y-%m-%d")
            .to_string())
    }

    fn fetch<T: serde::de::DeserializeOwned>(&self, endpoint: &str, date: &str) -> Result<Vec<T>> {
        let end = Self::next_day(date)?;
        let url = format!("https://api.ouraring.com/v2/usercollection/{endpoint}");
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("start_date", date), ("end_date", &end)])
            .send()
            .context("Failed to reach Oura API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("Oura API returned {status}: {body}");
        }

        let body: ApiResponse<T> = resp.json().context("Failed to parse API response")?;
        Ok(body.data)
    }

    pub fn daily_readiness(&self, date: &str) -> Result<Vec<DailyReadiness>> {
        self.fetch("daily_readiness", date)
    }

    pub fn daily_sleep(&self, date: &str) -> Result<Vec<DailySleep>> {
        self.fetch("daily_sleep", date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_day_happy_path() {
        assert_eq!(OuraClient::next_day("2026-01-15").unwrap(), "2026-01-16");
    }

    #[test]
    fn next_day_month_boundary() {
        assert_eq!(OuraClient::next_day("2026-01-31").unwrap(), "2026-02-01");
    }

    #[test]
    fn next_day_year_boundary() {
        assert_eq!(OuraClient::next_day("2026-12-31").unwrap(), "2027-01-01");
    }

    #[test]
    fn next_day_leap_year() {
        assert_eq!(OuraClient::next_day("2024-02-28").unwrap(), "2024-02-29");
        assert_eq!(OuraClient::next_day("2024-02-29").unwrap(), "2024-03-01");
    }

    #[test]
    fn next_day_non_leap_year() {
        assert_eq!(OuraClient::next_day("2025-02-28").unwrap(), "2025-03-01");
    }

    #[test]
    fn next_day_two_digit_year_is_year_26() {
        // chrono %Y does not require four digits, so "26-01-15" is 0026-01-15.
        assert_eq!(OuraClient::next_day("26-01-15").unwrap(), "0026-01-16");
    }

    #[test]
    fn next_day_accepts_unpadded_month_and_leading_space() {
        assert_eq!(OuraClient::next_day("2026-1-15").unwrap(), "2026-01-16");
        assert_eq!(OuraClient::next_day(" 2026-01-15").unwrap(), "2026-01-16");
    }

    #[test]
    fn next_day_rejects_malformed() {
        for bad in [
            "",
            "not-a-date",
            "2026/01/15",
            "2026-13-01",
            "2026-00-01",
            "2026-01-32",
            "2026-02-30",
            "2026-01-15T00:00:00",
            "2025-02-29",
            "2026-01-15 ",
            "2026-01",
        ] {
            assert!(
                OuraClient::next_day(bad).is_err(),
                "expected error for {bad}"
            );
        }
    }

    #[test]
    fn parse_readiness_happy_path() {
        let json = r#"{
            "data": [{
                "day": "2026-08-21",
                "score": 85,
                "contributors": {"hrv_balance": 72}
            }]
        }"#;
        let parsed: ApiResponse<DailyReadiness> = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.data.len(), 1);
        assert_eq!(parsed.data[0].score, 85);
        assert_eq!(parsed.data[0].contributors.hrv_balance, Some(72));
    }

    #[test]
    fn parse_readiness_missing_hrv_balance_is_none() {
        let json = r#"{
            "data": [{
                "day": "2026-08-21",
                "score": 40,
                "contributors": {}
            }]
        }"#;
        let parsed: ApiResponse<DailyReadiness> = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.data[0].contributors.hrv_balance, None);
    }

    #[test]
    fn parse_readiness_null_hrv_balance_is_none() {
        let json = r#"{
            "data": [{
                "day": "2026-08-21",
                "score": 40,
                "contributors": {"hrv_balance": null}
            }]
        }"#;
        let parsed: ApiResponse<DailyReadiness> = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.data[0].contributors.hrv_balance, None);
    }

    #[test]
    fn parse_readiness_empty_data() {
        let parsed: ApiResponse<DailyReadiness> = serde_json::from_str(r#"{"data": []}"#).unwrap();
        assert!(parsed.data.is_empty());
    }

    #[test]
    fn parse_readiness_ignores_extra_fields() {
        let json = r#"{
            "data": [{
                "day": "2026-08-21",
                "score": 10,
                "contributors": {"hrv_balance": 1, "resting_heart_rate": 9},
                "temperature_deviation": 0.2
            }],
            "next_token": "abc"
        }"#;
        let parsed: ApiResponse<DailyReadiness> = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.data[0].score, 10);
    }

    #[test]
    fn parse_readiness_missing_score_errors() {
        let json = r#"{"data": [{"day": "2026-08-21", "contributors": {}}]}"#;
        let parsed: Result<ApiResponse<DailyReadiness>, _> = serde_json::from_str(json);
        assert!(parsed.is_err());
    }

    #[test]
    fn parse_readiness_missing_contributors_errors() {
        let json = r#"{"data": [{"day": "2026-08-21", "score": 1}]}"#;
        let parsed: Result<ApiResponse<DailyReadiness>, _> = serde_json::from_str(json);
        assert!(parsed.is_err());
    }

    #[test]
    fn parse_sleep_happy_and_malformed() {
        let ok: ApiResponse<DailySleep> =
            serde_json::from_str(r#"{"data": [{"day": "2026-08-21", "score": 77}]}"#).unwrap();
        assert_eq!(ok.data[0].score, 77);

        let bad: Result<ApiResponse<DailySleep>, _> =
            serde_json::from_str(r#"{"data": [{"day": "2026-08-21"}]}"#);
        assert!(bad.is_err());
    }

    #[test]
    fn parse_api_response_not_json_errors() {
        let parsed: Result<ApiResponse<DailySleep>, _> = serde_json::from_str("nope");
        assert!(parsed.is_err());
    }
}
