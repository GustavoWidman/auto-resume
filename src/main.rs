mod chat;
mod latex;
mod models;
mod scraper;
mod utils;

use std::sync::Arc;
use rayon::prelude::*;

// Type alias for GitHub repository data: (name, url, stars, forks, size, importance_score, language, created_at, pushed_at, readme, commit_count)
type GitHubRepoData = (String, String, u64, u64, u64, u64, Option<String>, String, String, Option<String>, u64);

use clap::Parser;
use colored::Colorize;
use eyre::Result;
use log::{debug, info};
use tectonic::latex_to_pdf;

use crate::chat::agent::{ResumeAgent, resume_output_to_resume_items, RankedRepository};
use crate::latex::assembler::{LatexResumeAssembler, ResumeLanguage};
use crate::scraper::github::GitHubScraper;
use crate::scraper::job::JobScraper;
use crate::utils::cache;
use crate::utils::cli::Args;
use crate::utils::config::{Config, config};
use crate::utils::log::Logger;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    Logger::init(args.verbosity);

    info!(
        "starting auto-resume {}",
        format!("v{}", env!("CARGO_PKG_VERSION")).magenta()
    );

    let mut job_description = get_job_description(&args).await?;
    info!("job description loaded: {}", job_description.title);

    let config: Config = config(args.config)?;

    // Always process job description through AI agent to structure and extract data
    // (handles both HTML and plain text formats)
    info!("processing job description with LLM for consistency");
    let api_key = config
        .llm
        .api_key
        .clone()
        .ok_or_else(|| eyre::eyre!("LLM API key not configured in config.toml"))?;
    let agent = ResumeAgent::new(
        api_key,
        config.llm.model.clone(),
        config.llm.endpoint.clone(),
        config.llm.max_retries,
    );
    job_description = agent
        .clean_job_description(&job_description.description)
        .await?;
    info!("job description processed successfully");

    debug!("job description: {:#?}", job_description);

    let github_repos = scrape_github_profile(&config).await?;

    let language = parse_language(&args.language);

    let api_key = config
        .llm
        .api_key
        .clone()
        .ok_or_else(|| eyre::eyre!("LLM API key not configured in config.toml"))?;

    let agent = ResumeAgent::new(
        api_key,
        config.llm.model.clone(),
        config.llm.endpoint.clone(),
        config.llm.max_retries,
    );

    // Step 1: Rank repositories to help user select
    info!("ranking repositories based on job requirements");
    let ranked_repos = agent
        .rank_repositories(&github_repos, &job_description)
        .await?;

    // Step 2: Interactive selection
    let selected_repos = select_repositories_interactive(ranked_repos, &github_repos);
    info!("using {} selected repositories for resume generation", selected_repos.len());

    // Step 3: Generate resume with selected repositories
    let llm_output = agent
        .generate_resume_content(
            &config.resume,
            &job_description,
            selected_repos,
            match language {
                ResumeLanguage::English => "en",
                ResumeLanguage::Portuguese => "pt",
            },
        )
        .await?;

    let (skills, experience, projects, education) = resume_output_to_resume_items(&llm_output);

    let config = Arc::new({
        let mut cfg = (*config).clone();
        cfg.resume.skills = if skills.items.is_empty() {
            cfg.resume.skills
        } else {
            vec![skills]
        };
        cfg.resume.experience = if experience.is_empty() {
            cfg.resume.experience
        } else {
            experience
        };
        cfg.resume.projects = if projects.is_empty() {
            cfg.resume.projects
        } else {
            projects
        };
        cfg.resume.education = if education.is_empty() {
            cfg.resume.education
        } else {
            education
        };
        cfg
    });

    let latex = LatexResumeAssembler::new(config, language).assemble();

    info!("compiling LaTeX to PDF");
    let pdf = tokio::task::spawn_blocking(|| latex_to_pdf(latex))
        .await?
        .map_err(|e| {
            eprintln!("Tectonic error details: {:#?}", e);
            eyre::eyre!("failed to compile LaTeX document: {}", e.description())
        })?;

    info!("LaTeX compilation successful");
    tokio::fs::write(&args.output, pdf).await?;
    info!("generated resume at {}", args.output.display());

    Ok(())
}

async fn get_job_description(args: &Args) -> Result<crate::scraper::job::JobDescription> {
    if let Some(ref url) = args.job_url {
        JobScraper::from_url(url).await
    } else if let Some(ref file) = args.job_file {
        JobScraper::from_file(file).await
    } else {
        info!("no job description provided, using generic template");
        Ok(crate::scraper::job::JobDescription {
            title: "Software Engineer".to_string(),
            company: Some("Your Target Company".to_string()),
            description: "Software engineer position focused on building scalable systems and solving complex technical challenges.".to_string(),
            requirements: "Experience with modern software development practices, strong problem-solving skills, and collaborative mindset.".to_string(),
        })
    }
}

async fn scrape_github_profile(config: &Config) -> Result<Vec<GitHubRepoData>> {
    info!("scraping GitHub profile");

    cache::init_cache()?;

    let scraper = Arc::new(GitHubScraper::new(config.clone()));
    let repos = scraper.list_repositories().await?;

    if repos.is_empty() {
        info!("no public repositories found on GitHub profile");
        return Ok(Vec::new());
    }

    info!("fetching README and commit data for {} repositories (parallel)", repos.len());

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
            (repo, if readme.is_empty() { None } else { Some(readme) }, commits)
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
        .map(|(repo, readme, commits)| {
            (
                repo.name.clone(),
                repo.url.clone(),
                repo.stargazers_count,
                repo.forks_count,
                repo.size,
                repo.importance_score(),
                repo.language.clone(),
                repo.created_at.clone(),
                repo.pushed_at.clone(),
                readme,
                commits,
            )
        })
        .collect();

    info!("found {} repositories to analyze", result.len());
    Ok(result)
}

fn select_repositories_interactive(
    ranked: Vec<RankedRepository>,
    all_repos: &[GitHubRepoData],
) -> Vec<GitHubRepoData> {
    use std::io::{self, Write};

    println!("\n{}", "=== Repository Selection ===".cyan().bold());
    println!(
        "{}\n",
        "Select repositories to include in your resume (top 10 recommended):".cyan()
    );

    for repo in &ranked {
        let stars = "★".repeat(std::cmp::min(5, repo.rank / 2));
        println!(
            "{}. [{}] {}",
            repo.rank,
            stars.yellow(),
            repo.name.bold()
        );
        println!("   {}\n", repo.reasoning);
    }

    loop {
        print!(
            "{}",
            "Enter repository numbers to include (comma-separated, e.g., '1,2,3'): ".cyan()
        );
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("{}", "Error reading input. Please try again.".red());
            continue;
        }

        let input = input.trim();
        if input.is_empty() {
            println!("{}", "No repositories selected. Using top 5 by default.".yellow());
            return ranked
                .iter()
                .take(5)
                .filter_map(|r| {
                    all_repos.iter().find(|(name, ..)| name == &r.name).cloned()
                })
                .collect();
        }

        let mut selected_repos = Vec::new();
        let mut valid = true;

        for num_str in input.split(',') {
            match num_str.trim().parse::<usize>() {
                Ok(num) => {
                    if let Some(ranked_repo) = ranked.iter().find(|r| r.rank == num) {
                        if let Some(repo) = all_repos.iter().find(|(name, ..)| name == &ranked_repo.name) {
                            selected_repos.push(repo.clone());
                        }
                    } else {
                        println!("{}", format!("Invalid rank: {}. Please try again.", num).red());
                        valid = false;
                        break;
                    }
                }
                Err(_) => {
                    println!(
                        "{}",
                        format!("Invalid input: '{}'. Please enter numbers separated by commas.", num_str.trim()).red()
                    );
                    valid = false;
                    break;
                }
            }
        }

        if valid && !selected_repos.is_empty() {
            info!("selected {} repositories for resume", selected_repos.len());

            // Ask if user wants to add more repositories
            loop {
                print!("\n{}", "Add more repositories manually? (y/n): ".cyan());
                io::stdout().flush().unwrap();

                let mut add_more = String::new();
                if io::stdin().read_line(&mut add_more).is_err() {
                    println!("{}", "Error reading input. Please try again.".red());
                    continue;
                }

                match add_more.trim().to_lowercase().as_str() {
                    "y" | "yes" => {
                        println!("\n{}", "Add repositories by name (e.g., 'owner/repo-name' or just 'repo-name'):".cyan());
                        loop {
                            print!("{}", "Enter repository name(s) (comma-separated, or press Enter to finish): ".cyan());
                            io::stdout().flush().unwrap();

                            let mut manual_input = String::new();
                            if io::stdin().read_line(&mut manual_input).is_err() {
                                println!("{}", "Error reading input. Please try again.".red());
                                continue;
                            }

                            let manual_input = manual_input.trim();
                            if manual_input.is_empty() {
                                break;
                            }

                            for repo_name in manual_input.split(',') {
                                let repo_name = repo_name.trim();
                                if let Some(existing) = all_repos.iter().find(|(name, ..)| name == repo_name) {
                                    if !selected_repos.iter().any(|(name, ..)| name == &existing.0) {
                                        selected_repos.push(existing.clone());
                                        println!("{}", format!("✓ Added: {}", repo_name).green());
                                    } else {
                                        println!("{}", format!("⊘ Already selected: {}", repo_name).yellow());
                                    }
                                } else {
                                    println!("{}", format!("✗ Not found in profile: {}", repo_name).red());

                                    // Offer suggestions from available repos
                                    let similar: Vec<_> = all_repos
                                        .iter()
                                        .filter(|(name, ..)| {
                                            name.to_lowercase().contains(&repo_name.to_lowercase())
                                                || repo_name.to_lowercase().contains(&name.to_lowercase())
                                        })
                                        .take(3)
                                        .collect();

                                    if !similar.is_empty() {
                                        println!("  {} Did you mean:", "→".cyan());
                                        for (name, ..) in similar {
                                            println!("    • {}", name.cyan());
                                        }
                                    } else {
                                        println!("  {} No similar repos found. Available repos in your profile:", "→".cyan());
                                        for (name, ..) in all_repos.iter().take(5) {
                                            println!("    • {}", name.cyan());
                                        }
                                        if all_repos.len() > 5 {
                                            println!("    {} and {} more...", "•".cyan(), (all_repos.len() - 5).to_string().cyan());
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    }
                    "n" | "no" => break,
                    _ => {
                        println!("{}", "Please enter 'y' or 'n'.".red());
                    }
                }
            }

            info!("final selection: {} repositories for resume", selected_repos.len());
            return selected_repos;
        }
    }
}

fn parse_language(lang: &str) -> ResumeLanguage {
    match lang.to_lowercase().as_str() {
        "en" | "english" => ResumeLanguage::English,
        "pt" | "portuguese" => ResumeLanguage::Portuguese,
        _ => ResumeLanguage::English,
    }
}
