mod chat;
mod latex;
mod models;
mod scraper;
mod utils;

use std::io::{self, Write};
use std::sync::Arc;

use clap::Parser;
use colored::Colorize;
use eyre::Result;
use log::{debug, error, info};
use tectonic::latex_to_pdf;

use crate::chat::agent::{ResumeAgent, resume_output_to_resume_items};
use crate::latex::assembler::LatexResumeAssembler;
use crate::scraper::github::scrape_github_profile;
use crate::scraper::job::get_job_description;
use crate::utils::cli::Args;
use crate::utils::config::{Config, config};
use crate::utils::log::Logger;
use crate::utils::select_repos::select_repositories_interactive;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    Logger::init(args.verbosity);

    info!(
        "starting auto-resume {}",
        format!("v{}", env!("CARGO_PKG_VERSION")).magenta()
    );

    let config: Config = config(&args.config)?;

    let github_repos = scrape_github_profile(&config).await?;

    let job_description: String = get_job_description(&args).await?;
    debug!("job description loaded: {}", job_description);
    info!("processing job description with LLM for consistency");
    let agent = ResumeAgent::new(
        config
            .llm
            .api_key
            .clone()
            .ok_or_else(|| eyre::eyre!("LLM API key not configured in config.toml"))?,
        config.llm.model.clone(),
        config.llm.endpoint.clone(),
        config.llm.max_retries,
    );
    let job_description = agent.clean_job_description(&job_description).await?;
    info!(
        "job description processed successfully:\nTitle: {}\nDescription: {}\nRequirements: {}",
        job_description.title, job_description.description, job_description.requirements
    );

    info!("ranking repositories based on job requirements");
    let ranked_repos = agent
        .rank_repositories(&github_repos, &job_description)
        .await?;

    // Step 2: Interactive selection
    let selected_repos = select_repositories_interactive(ranked_repos, &github_repos);
    info!(
        "using {} selected repositories for resume generation",
        selected_repos.len()
    );

    let llm_output = agent
        .generate_resume_content(
            &config.resume,
            &job_description,
            selected_repos,
            &args.language,
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

    let mut latex = LatexResumeAssembler::new(config, args.language).assemble();

    info!("would you like to edit the generated LaTeX source before compiling? (y/N): ");
    latex = loop {
        io::stdout().flush().unwrap();

        let mut edit = String::new();
        if io::stdin().read_line(&mut edit).is_err() {
            println!("{}", "Error reading input. Please try again.".red());
            continue;
        }

        match edit.trim().to_lowercase().as_str() {
            "y" | "yes" => {
                let temp_path = std::env::temp_dir().join("resume.tex");
                tokio::fs::write(&temp_path, latex.clone()).await?;
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                std::process::Command::new(editor)
                    .arg(&temp_path)
                    .status()
                    .expect("failed to open editor");
                let edited_latex = tokio::fs::read_to_string(&temp_path).await?;
                tokio::fs::remove_file(&temp_path).await?;
                break edited_latex;
            }
            "n" | "no" | "" => break latex,
            _ => error!("invalid input. please enter 'y' or 'n'."),
        }
    };

    if args.latex {
        info!(
            "saving intermediate LaTeX source to {}",
            args.output.with_extension("tex").display()
        );
        tokio::fs::write(args.output.with_extension("tex"), latex.clone()).await?;
    }

    info!("compiling LaTeX to PDF");
    let pdf = tokio::task::spawn_blocking(|| latex_to_pdf(latex))
        .await?
        .map_err(|e| {
            eprintln!("Tectonic error details: {:#?}", e);
            eyre::eyre!("failed to compile LaTeX document: {}", e.description())
        })?;

    tokio::fs::write(&args.output, pdf).await?;
    info!("generated resume at {}", args.output.display());

    Ok(())
}
