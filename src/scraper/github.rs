use std::sync::Arc;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use eyre::Result;
use log::{debug, info, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::models::github::{Repository, RepositoryLanguages};
use crate::utils::cache;
use crate::utils::config::Config;

#[derive(Clone, Debug)]
pub struct GitHubRepoData {
    pub name: String,
    pub url: String,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub size: u64,
    pub importance_score: u64,
    pub languages: Option<RepositoryLanguages>,
    pub created_at: String,
    pub pushed_at: String,
    pub readme: Option<String>,
    pub commits: u64,
}

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
        Ok(self
            .list_repositories_internal()
            .await?
            .into_iter()
            .filter(|repo| !repo.fork)
            .collect())
        .map(|mut repos: Vec<Repository>| {
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

        let pattern = regex::Regex::new(
            r#"<https://api\.github\.com/repositories/\d+/commits\?per_page=1&page=2>; rel="next", <https://api\.github\.com/repositories/\d+/commits\?per_page=1&page=(\d+)>; rel="last""#,
        )?;
        if let Some(link_header) = response.headers().get("link")
            && let Some(captures) = pattern.captures(link_header.to_str()?)
            && let Some(last_page_match) = captures.get(1)
            && let Ok(last_page) = last_page_match.as_str().parse::<u64>()
        {
            return Ok(last_page);
        }

        warn!("Could not determine commit count for repo: {}", repo.name);

        Ok(0)
    }

    pub async fn get_languages(&self, repo: &Repository) -> Result<RepositoryLanguages> {
        let mut req = self
            .client
            .get(format!("{}/languages", repo.url))
            .header("User-Agent", "auto-resume-app");
        if let Some(token) = &self.config.github.token {
            req = req.header("Authorization", format!("token {}", token));
        }

        let response = req.send().await?;

        let languages: RepositoryLanguages = response.json().await?;

        Ok(languages)
    }
}

pub async fn scrape_github_profile(config: &Config) -> Result<Vec<GitHubRepoData>> {
    info!("scraping GitHub profile");

    cache::init_cache()?;

    let scraper = Arc::new(GitHubScraper::new(config.clone()));
    let repos = scraper.list_repositories().await?;

    if repos.is_empty() {
        info!("no public repositories found on GitHub profile");
        return Ok(Vec::new());
    }

    info!(
        "fetching README and commit data for {} repositories (parallel)",
        repos.len()
    );

    // Create concurrent tasks for fetching README and commits for each repo
    let mut tasks = Vec::new();
    for repo in repos.into_iter() {
        let scraper_clone = Arc::clone(&scraper);
        let task = tokio::spawn(async move {
            // Check cache first for README
            let readme = if let Some(cached) = cache::get_cached_readme(&repo.name) {
                cached
            } else {
                match scraper_clone.get_readme(&repo).await {
                    Ok(Some(content)) => {
                        let _ = cache::cache_readme(&repo.name, &content);
                        content
                    }
                    _ => String::new(),
                }
            };

            let commits = scraper_clone.get_commit_count(&repo).await.unwrap_or(0);

            let languages = scraper_clone.get_languages(&repo).await.ok();

            (
                repo,
                if readme.is_empty() {
                    None
                } else {
                    Some(readme)
                },
                commits,
                languages,
            )
        });
        tasks.push(task);
    }

    // Await all tasks and collect results in parallel using rayon
    let mut completed_tasks = Vec::with_capacity(tasks.len());
    for task in tasks {
        completed_tasks.push(task.await?);
    }

    let result: Vec<GitHubRepoData> = completed_tasks
        .into_par_iter()
        .map(|(repo, readme, commits, languages)| GitHubRepoData {
            name: repo.name.clone(),
            url: repo.url.clone(),
            stargazers_count: repo.stargazers_count,
            forks_count: repo.forks_count,
            size: repo.size,
            importance_score: repo.importance_score(),
            languages,
            created_at: repo.created_at.clone(),
            pushed_at: repo.pushed_at.clone(),
            readme,
            commits,
        })
        .collect();

    info!("found {} repositories to analyze", result.len());
    Ok(result)
}
