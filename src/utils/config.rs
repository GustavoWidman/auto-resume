use std::path::PathBuf;
use std::sync::Arc;

use easy_config_store::ConfigStore;
use eyre::Result;
use log::{debug, info};
use serde::{Deserialize, Serialize};

pub type Config = Arc<ConfigInner>;

pub fn config(path: PathBuf) -> Result<Config> {
    let config_store = ConfigStore::<ConfigInner>::read(path, "config".to_string())?;
    let inner = (*config_store).clone();

    info!("config parsing successful");
    debug!("loaded configuration:\n{}", toml::to_string_pretty(&inner)?);

    Ok(Arc::new(inner))
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ConfigInner {
    pub resume: ResumeConfig,
    pub github: GithubConfig,
    pub llm: LLMConfig,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ResumeConfig {
    pub full_name: String,
    pub country: String,
    pub city: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub linkedin: Option<String>,
    pub github: Option<String>,
    pub site: Option<String>,
    #[serde(default)]
    pub education: Vec<ResumeItem>,
    #[serde(default)]
    pub skills: Vec<ResumeItem>,
    #[serde(default)]
    pub experience: Vec<ResumeItem>,
    #[serde(default)]
    pub projects: Vec<ResumeItem>,
    pub education_context: Option<String>,
    pub experience_context: Option<String>,
    pub skills_context: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ResumeItem {
    pub title: Option<String>,
    pub date: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
    pub items: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct GithubConfig {
    pub token: Option<String>,
    pub username: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct LLMConfig {
    pub api_key: Option<String>,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_llm_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_llm_model() -> String {
    "gemini-2.0-flash-exp".to_string()
}

fn default_llm_endpoint() -> String {
    "https://generativelanguage.googleapis.com/v1beta/models".to_string()
}

fn default_max_retries() -> u32 {
    3
}

impl Default for ConfigInner {
    fn default() -> Self {
        let cfg = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.default.toml",));

        toml::from_str(cfg).unwrap() // should be okay
    }
}
