use std::io::{self, Write};

use colored::Colorize;
use log::info;

use crate::chat::agent::RankedRepository;
use crate::scraper::github::GitHubRepoData;

pub fn select_repositories_interactive(
    ranked: Vec<RankedRepository>,
    all_repos: &[GitHubRepoData],
) -> Vec<GitHubRepoData> {
    println!("\n{}", "=== Repository Selection ===".cyan().bold());
    println!(
        "{}\n",
        "Select repositories to include in your resume (top 10 recommended):".cyan()
    );

    for repo in &ranked {
        let stars = "★".repeat(std::cmp::min(5, repo.rank / 2));
        println!("{}. [{}] {}", repo.rank, stars.yellow(), repo.name.bold());
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
            println!(
                "{}",
                "No repositories selected. Using top 5 by default.".yellow()
            );
            return ranked
                .iter()
                .take(5)
                .filter_map(|r| all_repos.iter().find(|repo| repo.name == r.name).cloned())
                .collect();
        }

        let mut selected_repos = Vec::new();
        let mut valid = true;

        for num_str in input.split(',') {
            match num_str.trim().parse::<usize>() {
                Ok(num) => {
                    if let Some(ranked_repo) = ranked.iter().find(|r| r.rank == num) {
                        if let Some(repo) =
                            all_repos.iter().find(|repo| repo.name == ranked_repo.name)
                        {
                            selected_repos.push(repo.clone());
                        }
                    } else {
                        println!(
                            "{}",
                            format!("Invalid rank: {}. Please try again.", num).red()
                        );
                        valid = false;
                        break;
                    }
                }
                Err(_) => {
                    println!(
                        "{}",
                        format!(
                            "Invalid input: '{}'. Please enter numbers separated by commas.",
                            num_str.trim()
                        )
                        .red()
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
                                if let Some(existing) =
                                    all_repos.iter().find(|repo| repo.name == repo_name)
                                {
                                    if !selected_repos.iter().any(|repo| repo.name == existing.name)
                                    {
                                        selected_repos.push(existing.clone());
                                        println!("{}", format!("✓ Added: {}", repo_name).green());
                                    } else {
                                        println!(
                                            "{}",
                                            format!("⊘ Already selected: {}", repo_name).yellow()
                                        );
                                    }
                                } else {
                                    println!(
                                        "{}",
                                        format!("✗ Not found in profile: {}", repo_name).red()
                                    );

                                    // Offer suggestions from available repos
                                    let similar: Vec<_> = all_repos
                                        .iter()
                                        .filter(|repo| {
                                            repo.name
                                                .to_lowercase()
                                                .contains(&repo_name.to_lowercase())
                                                || repo_name
                                                    .to_lowercase()
                                                    .contains(&repo.name.to_lowercase())
                                        })
                                        .take(3)
                                        .collect();

                                    if !similar.is_empty() {
                                        println!("  {} Did you mean:", "→".cyan());
                                        for repo in similar {
                                            println!("    • {}", repo.name.cyan());
                                        }
                                    } else {
                                        println!(
                                            "  {} No similar repos found. Available repos in your profile:",
                                            "→".cyan()
                                        );
                                        for repo in all_repos.iter().take(5) {
                                            println!("    • {}", repo.name.cyan());
                                        }
                                        if all_repos.len() > 5 {
                                            println!(
                                                "    {} and {} more...",
                                                "•".cyan(),
                                                (all_repos.len() - 5).to_string().cyan()
                                            );
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

            info!(
                "final selection: {} repositories for resume",
                selected_repos.len()
            );
            return selected_repos;
        }
    }
}
