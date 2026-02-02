use backon::{ExponentialBuilder, Retryable};
use eyre::{Result, eyre};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::scraper::job::JobDescription;
use crate::utils::config::{ResumeConfig, ResumeItem};

// Type alias for GitHub repository data: (name, url, stars, forks, size, importance_score, language, created_at, pushed_at, readme, commit_count)
type GitHubRepoData = (String, String, u64, u64, u64, u64, Option<String>, String, String, Option<String>, u64);

#[derive(Debug, Clone)]
pub struct RankedRepository {
    pub rank: usize,
    pub name: String,
    pub reasoning: String,
}

const SYSTEM_PROMPT: &str = include_str!("system_prompt.txt");
const PROMPT_TEMPLATE: &str = include_str!("prompt_template.txt");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResumeOutput {
    pub skills_by_category: Vec<SkillCategory>,
    pub projects: Vec<ProjectEntry>,
    pub education: Vec<EducationEntry>,
    pub experience: Vec<ExperienceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EducationEntry {
    pub institution: String,
    pub degree: String,
    pub location: String,
    pub date: String,
    pub accomplishments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceEntry {
    pub company: String,
    pub position: String,
    pub location: String,
    pub date: String,
    pub accomplishments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCategory {
    pub category: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub title: String,
    pub link: String,
    pub items: Vec<String>,
}

pub struct ResumeAgent {
    api_key: String,
    model: String,
    endpoint: String,
    max_retries: u32,
}

impl ResumeAgent {
    pub fn new(api_key: String, model: String, endpoint: String, max_retries: u32) -> Self {
        Self {
            api_key,
            model,
            endpoint,
            max_retries,
        }
    }

    pub async fn clean_job_description(&self, raw_html_or_text: &str) -> Result<JobDescription> {
        info!(
            "cleaning job description using LLM (max retries: {})",
            self.max_retries
        );
        debug!("job description content length: {}", raw_html_or_text.len());

        let prompt = format!(
            "Extract and clean the job description from the following content. \
            If it's HTML, convert to plain text. If it's already plain text, clean it up.\n\n\
            IMPORTANT: Respond ONLY with valid JSON in this format:\n\
            {{\n\
              \"title\": \"Job Title\",\n\
              \"company\": \"Company Name\",\n\
              \"description\": \"Clean job description with key responsibilities\",\n\
              \"requirements\": \"Key technical requirements and qualifications\"\n\
            }}\n\n\
            Content to process:\n{}",
            raw_html_or_text
        );

        let client = reqwest::Client::new();
        let api_key = self.api_key.clone();
        let endpoint = self.endpoint.clone();
        let model = self.model.clone();

        let result = (|| async {
            let request_body = json!({
                "contents": [{"parts": [{"text": prompt}]}],
                "generationConfig": {
                    "temperature": 0.3,
                    "maxOutputTokens": 2048,
                    "responseMimeType": "application/json",
                    "responseJsonSchema": {
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "Job title/position name"
                            },
                            "company": {
                                "type": "string",
                                "description": "Company name (can be null if not found)"
                            },
                            "description": {
                                "type": "string",
                                "description": "Clean job description with key responsibilities"
                            },
                            "requirements": {
                                "type": "string",
                                "description": "Key technical requirements and qualifications"
                            }
                        },
                        "required": ["title", "description", "requirements"]
                    }
                }
            });

            let url = format!(
                "{}/{}:generateContent?key={}",
                endpoint.trim_end_matches('/'),
                model,
                api_key
            );

            let response = client.post(&url).json(&request_body).send().await?;

            if !response.status().is_success() {
                let error = response.text().await?;
                return Err(eyre!("LLM job cleaning failed: {}", error));
            }

            Ok(response)
        })
        .retry(ExponentialBuilder::default())
        .await?;

        let body: serde_json::Value = result.json().await?;

        let content = body
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| eyre!("invalid LLM response for job cleaning"))?;

        let trimmed = content.trim();
        let json_start = trimmed
            .find('{')
            .ok_or_else(|| eyre!("no JSON found in job cleaning response"))?;
        let json_end = trimmed
            .rfind('}')
            .ok_or_else(|| eyre!("malformed JSON in job cleaning response"))?;
        let json_str = &trimmed[json_start..=json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)?;

        Ok(JobDescription {
            title: parsed["title"].as_str().unwrap_or("Job Title").to_string(),
            company: parsed["company"].as_str().map(|s| s.to_string()),
            description: parsed["description"].as_str().unwrap_or("").to_string(),
            requirements: parsed["requirements"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn rank_repositories(
        &self,
        github_repos: &[GitHubRepoData],
        job_description: &JobDescription,
    ) -> Result<Vec<RankedRepository>> {
        info!("ranking repositories to help with selection");
        debug!("evaluating {} repositories", github_repos.len());

        let repos_list = github_repos
            .iter()
            .map(|(name, url, stars, forks, _size, score, lang, created_at, pushed_at, readme, commits)| {
                let lang_str = lang.as_deref().unwrap_or("Unknown");
                let created_year = created_at.split('-').next().unwrap_or("????");
                let pushed_year = pushed_at.split('-').next().unwrap_or("????");
                let readme_indicator = if readme.is_some() { " [HAS_README]" } else { "" };
                format!(
                    "- {} [{}] (created: {}, last updated: {}, stars: {}, forks: {}, commits: {}, importance: {}){}  {}",
                    name, lang_str, created_year, pushed_year, stars, forks, commits, score, readme_indicator, url
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Rank and select the BEST 10 repositories from this candidate's GitHub profile that would be most impressive for a resume targeting a {} role at {}.\n\n\
            CRITERIA:\n\
            - Recent activity (prefer repos updated in last 2 years)\n\
            - Maintenance status (avoid abandoned projects)\n\
            - Language diversity (vary the tech stack)\n\
            - Relevance to job requirements: {}\n\
            - Project maturity (complete, not WIP)\n\
            - Star count and forks (community engagement)\n\n\
            REPOSITORIES:\n\
            {}\n\n\
            Respond ONLY with valid JSON in this exact format:\n\
            {{\n\
              \"ranked_repositories\": [\n\
                {{\n\
                  \"rank\": 1,\n\
                  \"name\": \"project-name\",\n\
                  \"reasoning\": \"Why this is a good choice for the resume\"\n\
                }},\n\
                ...\n\
              ]\n\
            }}",
            job_description.title,
            job_description.company.as_deref().unwrap_or("the target company"),
            job_description.description,
            repos_list
        );

        let client = reqwest::Client::new();
        let api_key = self.api_key.clone();
        let endpoint = self.endpoint.clone();
        let model = self.model.clone();

        let response = (|| async {
            let request_body = json!({
                "contents": [{"parts": [{"text": prompt}]}],
                "generationConfig": {
                    "temperature": 0.7,
                    "maxOutputTokens": 3000,
                    "responseMimeType": "application/json",
                    "responseJsonSchema": {
                        "type": "object",
                        "properties": {
                            "ranked_repositories": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "rank": { "type": "integer" },
                                        "name": { "type": "string" },
                                        "reasoning": { "type": "string" }
                                    },
                                    "required": ["rank", "name", "reasoning"]
                                }
                            }
                        },
                        "required": ["ranked_repositories"]
                    }
                }
            });

            let url = format!(
                "{}/{}:generateContent?key={}",
                endpoint.trim_end_matches('/'),
                model,
                api_key
            );

            let response = client.post(&url).json(&request_body).send().await?;

            let status = response.status();
            if !status.is_success() {
                let error_body = response.text().await?;
                return Err(eyre!("Repository ranking failed ({}): {}", status, error_body));
            }

            Ok(response)
        })
        .retry(ExponentialBuilder::default())
        .await?;

        let body: serde_json::Value = response.json().await?;

        let ranked = body
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| eyre!("invalid ranking response structure"))?;

        let trimmed = ranked.trim();
        let json_start = trimmed.find('{').ok_or_else(|| eyre!("no JSON in ranking response"))?;
        let json_end = trimmed.rfind('}').ok_or_else(|| eyre!("malformed JSON in ranking response"))?;
        let json_str = &trimmed[json_start..=json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            debug!("JSON parsing failed: {}", e);
            eyre!("failed to parse ranking response: {}", e)
        })?;

        let mut repositories = Vec::new();
        if let Some(ranked_repos) = parsed.get("ranked_repositories").and_then(|r| r.as_array()) {
            for repo in ranked_repos {
                if let (Some(rank), Some(name), Some(reasoning)) = (
                    repo.get("rank").and_then(|r| r.as_u64()),
                    repo.get("name").and_then(|n| n.as_str()),
                    repo.get("reasoning").and_then(|r| r.as_str()),
                ) {
                    repositories.push(RankedRepository {
                        rank: rank as usize,
                        name: name.to_string(),
                        reasoning: reasoning.to_string(),
                    });
                }
            }
        }

        info!("ranked {} repositories", repositories.len());
        Ok(repositories)
    }

    pub async fn generate_resume_content(
        &self,
        resume_config: &ResumeConfig,
        job_description: &JobDescription,
        github_repos: Vec<GitHubRepoData>,
        language: &str,
    ) -> Result<LLMResumeOutput> {
        info!("generating resume content using LLM with structured output");

        let prompt = self.build_prompt(resume_config, job_description, &github_repos, language);

        let response = self.call_gemini_api(&prompt).await?;

        let output = self.parse_response(&response)?;

        info!("successfully generated resume content");
        debug!("LLM output: {:#?}", output);

        Ok(output)
    }

    fn build_prompt(
        &self,
        resume_config: &ResumeConfig,
        job_description: &JobDescription,
        github_repos: &[GitHubRepoData],
        language: &str,
    ) -> String {
        let repos_list = github_repos
            .iter()
            .map(|(name, url, stars, forks, _size, score, lang, created_at, pushed_at, readme, commits)| {
                let lang_str = lang.as_deref().unwrap_or("Unknown");
                // Extract year from ISO date string (e.g., "2023-01-15T..." -> "2023")
                let created_year = created_at.split('-').next().unwrap_or("????");
                let pushed_year = pushed_at.split('-').next().unwrap_or("????");

                let readme_snippet = if let Some(readme_content) = readme {
                    // Show first 200 chars of README as snippet
                    let snippet = if readme_content.len() > 200 {
                        format!("{}...", &readme_content[..200])
                    } else {
                        readme_content.clone()
                    };
                    format!("\n  README: {}", snippet)
                } else {
                    String::new()
                };

                format!(
                    "- {} [{}] (created: {}, last updated: {}, stars: {}, forks: {}, commits: {}, importance: {}) - {}{}",
                    name, lang_str, created_year, pushed_year, stars, forks, commits, score, url, readme_snippet
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let education_context = resume_config
            .education_context
            .as_deref()
            .unwrap_or("User has not provided specific education details");

        let experience_context = resume_config
            .experience_context
            .as_deref()
            .unwrap_or("User has not provided specific experience details");

        let skills_context = resume_config
            .skills_context
            .as_deref()
            .unwrap_or("User has not provided specific skill details");

        PROMPT_TEMPLATE
            .replace("{candidate_name}", &resume_config.full_name)
            .replace("{job_title}", &job_description.title)
            .replace(
                "{job_company}",
                job_description
                    .company
                    .as_deref()
                    .unwrap_or("Unknown Company"),
            )
            .replace("{job_description}", &job_description.description)
            .replace("{github_repos}", &repos_list)
            .replace("{education_context}", education_context)
            .replace("{experience_context}", experience_context)
            .replace("{skills_context}", skills_context)
            .replace(
                "{language}",
                match language {
                    "en" => "English",
                    "pt" => "Portuguese",
                    _ => "English",
                },
            )
    }

    async fn call_gemini_api(&self, prompt: &str) -> Result<String> {
        info!(
            "calling Gemini API with structured output (model: {}, max retries: {})",
            self.model, self.max_retries
        );
        debug!("prompt length: {} characters", prompt.len());

        let client = reqwest::Client::new();
        let api_key = self.api_key.clone();
        let endpoint = self.endpoint.clone();
        let model = self.model.clone();

        let response = (|| async {
            let request_body = json!({
                "contents": [
                    {
                        "parts": [
                            {
                                "text": prompt
                            }
                        ]
                    }
                ],
                "systemInstruction": {
                    "parts": [
                        {
                            "text": SYSTEM_PROMPT
                        }
                    ]
                },
                "generationConfig": {
                    "temperature": 1.0,
                    "topP": 0.95,
                    "topK": 40,
                    "maxOutputTokens": 4096,
                    "responseMimeType": "application/json",
                    "responseJsonSchema": {
                        "type": "object",
                        "properties": {
                            "skills_by_category": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "category": {
                                            "type": "string",
                                            "description": "Technical skill category (e.g., Back-end, Front-end)"
                                        },
                                        "items": {
                                            "type": "array",
                                            "items": { "type": "string" },
                                            "description": "List of specific skills in this category"
                                        }
                                    },
                                    "required": ["category", "items"]
                                }
                            },
                            "projects": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "title": {
                                            "type": "string",
                                            "description": "Project name in format: 'Project Name (Technology/Language)', use parentheses instead of brackets."
                                        },
                                        "link": {
                                            "type": "string",
                                            "description": "GitHub repository URL"
                                        },
                                        "items": {
                                            "type": "array",
                                            "items": { "type": "string" },
                                            "description": "Single brief line (max 15 words) describing core purpose or key feature"
                                        }
                                    },
                                    "required": ["title", "link", "items"]
                                }
                            },
                            "education": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "institution": { "type": "string" },
                                        "degree": { "type": "string" },
                                        "location": { "type": "string" },
                                        "date": { "type": "string" },
                                        "accomplishments": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        }
                                    },
                                    "required": ["institution", "degree", "location", "date", "accomplishments"]
                                }
                            },
                            "experience": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "company": { "type": "string" },
                                        "position": { "type": "string" },
                                        "location": { "type": "string" },
                                        "date": { "type": "string" },
                                        "accomplishments": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        }
                                    },
                                    "required": ["company", "position", "location", "date", "accomplishments"]
                                }
                            }
                        },
                        "required": ["skills_by_category", "projects", "education", "experience"]
                    }
                }
            });

            let url = format!(
                "{}/{}:generateContent?key={}",
                endpoint.trim_end_matches('/'),
                model,
                api_key
            );

            let response = client.post(&url).json(&request_body).send().await?;

            let status = response.status();
            if !status.is_success() {
                let error_body = response.text().await?;
                return Err(eyre!("Gemini API error ({}): {}", status, error_body));
            }

            Ok(response)
        })
        .retry(ExponentialBuilder::default())
        .await?;

        let body: serde_json::Value = response.json().await?;

        let content = body
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| eyre!("invalid Gemini API response structure"))?;

        Ok(content.to_string())
    }

    fn parse_response(&self, response: &str) -> Result<LLMResumeOutput> {
        debug!("parsing LLM response (length: {} chars)", response.len());

        let trimmed = response.trim();
        let json_start = trimmed
            .find('{')
            .ok_or_else(|| eyre!("no JSON object found in response"))?;
        let json_end = trimmed
            .rfind('}')
            .ok_or_else(|| eyre!("malformed JSON in response"))?;

        let json_str = &trimmed[json_start..=json_end];
        debug!(
            "extracted JSON (length: {} chars): {}",
            json_str.len(),
            &json_str[..std::cmp::min(200, json_str.len())]
        );

        let output: LLMResumeOutput = serde_json::from_str(json_str).map_err(|e| {
            debug!("JSON parsing failed: {}", e);
            eyre!("failed to parse LLM response as JSON: {}", e)
        })?;

        Ok(output)
    }
}

pub fn resume_output_to_resume_items(
    output: &LLMResumeOutput,
) -> (
    ResumeItem,
    Vec<ResumeItem>,
    Vec<ResumeItem>,
    Vec<ResumeItem>,
) {
    let skills = ResumeItem {
        title: None,
        date: None,
        location: None,
        description: None,
        link: None,
        items: output
            .skills_by_category
            .iter()
            .map(|cat| format!("**{}**: {}", cat.category, cat.items.join(", ")))
            .collect(),
    };

    let projects: Vec<ResumeItem> = output
        .projects
        .iter()
        .map(|proj| ResumeItem {
            title: Some(proj.title.clone()),
            date: None,
            location: Some("GitHub".to_string()),
            description: None,
            link: Some(proj.link.clone()),
            items: proj.items.clone(),
        })
        .collect();

    let education: Vec<ResumeItem> = output
        .education
        .iter()
        .map(|edu| ResumeItem {
            title: Some(edu.institution.clone()),
            date: Some(edu.date.clone()),
            location: Some(edu.location.clone()),
            description: Some(edu.degree.clone()),
            link: None,
            items: edu.accomplishments.clone(),
        })
        .collect();

    let experience: Vec<ResumeItem> = output
        .experience
        .iter()
        .map(|exp| ResumeItem {
            title: Some(exp.company.clone()),
            date: Some(exp.date.clone()),
            location: Some(exp.location.clone()),
            description: Some(exp.position.clone()),
            link: None,
            items: exp.accomplishments.clone(),
        })
        .collect();

    (skills, experience, projects, education)
}
