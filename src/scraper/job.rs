use std::path::Path;

use eyre::Result;
use log::{info, warn};

use crate::utils::cli::Args;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct JobDescription {
    pub title: String,
    pub company: Option<String>,
    pub description: String,
    pub requirements: String,
}

pub struct JobScraper;

impl JobScraper {
    pub async fn from_url(url: &str) -> Result<String> {
        info!("fetching job description from: {}", url);

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send()
            .await?;

        let html = response.text().await?;

        info!("successfully extracted job description html");

        Ok(html)
    }

    pub async fn from_file(path: &Path) -> Result<String> {
        info!("reading job description from file: {}", path.display());

        tokio::fs::read_to_string(path).await.map_err(Into::into)
    }
}

pub async fn get_job_description(args: &Args) -> Result<String> {
    if let Some(ref url) = args.job_url {
        JobScraper::from_url(url).await
    } else if let Some(ref file) = args.job_file {
        JobScraper::from_file(file).await
    } else {
        warn!("no job description provided, using generic software engineer template");

        Ok(
            "Software Engineer position focused on building scalable systems and solving complex technical challenges. Experience with modern software development practices, strong problem-solving skills, and collaborative mindset.".to_string()
        )
    }
}
