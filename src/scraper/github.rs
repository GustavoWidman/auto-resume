use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use eyre::Result;
use log::{debug, info};

use crate::models::github::Repository;
use crate::utils::config::Config;

pub struct GitHubScraper {
    config: Config,
    client: reqwest::Client,
}

impl GitHubScraper {
    pub fn new(config: Config) -> Self {
        GitHubScraper {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        self.list_repositories_internal().await.map(|mut repos| {
            repos.sort_by_key(|b| std::cmp::Reverse(b.importance_score()));
            repos
        })
    }

    async fn list_repositories_internal(&self) -> Result<Vec<Repository>> {
        let mut page: u32 = 1;
        let mut repositories = Vec::new();
        loop {
            let mut req = self
                .client
                .get(format!(
                    "https://api.github.com/users/{}/repos?per_page=100&page={}",
                    self.config.github.username, page
                ))
                .header("User-Agent", "auto-resume-app");

            if let Some(token) = &self.config.github.token {
                req = req.header("Authorization", format!("token {}", token));
            }

            let response = req.send().await?;

            let has_next = response.headers().contains_key("link");
            repositories.append(&mut response.json().await?);

            match has_next {
                true => page += 1,
                false => break,
            }
        }

        Ok(repositories)
    }

    pub async fn get_readme(&self, repo: &Repository) -> Result<Option<String>> {
        let mut req = self
            .client
            .get(format!("{}/readme", repo.url))
            .header("User-Agent", "auto-resume-app");
        if let Some(token) = &self.config.github.token {
            req = req.header("Authorization", format!("token {}", token));
        }

        let response = req.send().await?;

        if response.status().is_success() {
            let readme: serde_json::Value = response.json().await?;
            if let Some(content) = readme.get("content") {
                info!("Found README for repo: {}", repo.name);
                debug!("Raw README content: {:?}", content);
                let decoded = BASE64_STANDARD
                    .decode(content.as_str().unwrap_or("").replace(['\n', '\r'], ""))?;
                let readme_str = String::from_utf8(decoded)?;
                return Ok(Some(readme_str));
            }
        }

        Ok(None)
    }

    pub async fn get_commit_count(&self, repo: &Repository) -> Result<u64> {
        let mut req = self
            .client
            .get(format!("{}/commits?per_page=1", repo.url))
            .header("User-Agent", "auto-resume-app");
        if let Some(token) = &self.config.github.token {
            req = req.header("Authorization", format!("token {}", token));
        }

        let response = req.send().await?;

        // The total count is in the Link header's last page URL
        if let Some(link_header) = response.headers().get("link")
            && let Ok(link_str) = link_header.to_str()
            && let Some(last_url) = link_str.split(',').find(|s| s.contains("rel=\"last\""))
            && let Some(page_str) = last_url.split("page=").nth(1)
            && let Ok(last_page) = page_str.split('>').next().unwrap_or("0").parse::<u64>()
        {
            return Ok(last_page);
        }

        // Fallback: return 0 if we can't determine
        Ok(0)
    }

    #[allow(dead_code)]
    pub async fn scrape(&self) -> Result<()> {
        let repos = self.list_repositories().await?;

        for repo in repos {
            info!("Repo: {}", repo.name);
            if let Some(readme) = self.get_readme(&repo).await? {
                info!("README:\n{}", readme);
            } else {
                info!("No README found.");
            }
        }

        Ok(())
    }
}
