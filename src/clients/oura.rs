use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DailyReadiness {
    pub day: String,
    pub score: u32,
    pub contributors: ReadinessContributors,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReadinessContributors {
    pub hrv_balance: Option<u32>,
}

#[derive(Debug, Deserialize)]
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
