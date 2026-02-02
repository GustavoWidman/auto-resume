use std::path::Path;

use eyre::Result;
use log::info;

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
    pub async fn from_url(url: &str) -> Result<JobDescription> {
        info!("fetching job description from: {}", url);

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send()
            .await?;

        let html = response.text().await?;
        let description = Self::parse_html(&html)?;

        info!("successfully extracted job description");
        Ok(description)
    }

    pub async fn from_file(path: &Path) -> Result<JobDescription> {
        info!("reading job description from file: {}", path.display());

        let content = tokio::fs::read_to_string(path).await?;
        Ok(JobDescription {
            title: String::from("Job Description"),
            company: None,
            description: content.clone(),
            requirements: content,
        })
    }

    fn parse_html(html: &str) -> Result<JobDescription> {
        let description = Self::extract_text_from_html(html);

        let title = Self::extract_job_title(html).unwrap_or_else(|| "Unknown Position".to_string());
        let company = Self::extract_company_name(html);
        let requirements = Self::extract_requirements(html).unwrap_or_else(|| description.clone());

        Ok(JobDescription {
            title,
            company,
            description,
            requirements,
        })
    }

    fn extract_text_from_html(html: &str) -> String {
        html.lines()
            .map(|line| {
                line.replace("<[^>]+>", "")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
                    .replace("&#39;", "'")
                    .trim()
                    .to_string()
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn extract_job_title(html: &str) -> Option<String> {
        let patterns = vec![
            r#"<h1[^>]*>([^<]+)</h1>"#,
            r#"<meta property="og:title" content="([^"]+)"#,
            r#"<title>([^<]+)</title>"#,
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern)
                && let Some(caps) = re.captures(html)
                && let Some(title) = caps.get(1)
            {
                return Some(title.as_str().to_string());
            }
        }
        None
    }

    fn extract_company_name(html: &str) -> Option<String> {
        let patterns = vec![
            r#"<meta property="og:site_name" content="([^"]+)"#,
            r#"company[^>]*>([^<]+)<"#,
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern)
                && let Some(caps) = re.captures(html)
                && let Some(company) = caps.get(1)
            {
                return Some(company.as_str().to_string());
            }
        }
        None
    }

    fn extract_requirements(html: &str) -> Option<String> {
        let patterns =
            vec![r#"(?i)(requirements|qualifications|skills|must-haves?)[:\s]*([^<]{100,})"#];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern)
                && let Some(caps) = re.captures(html)
                && let Some(req) = caps.get(2)
            {
                return Some(req.as_str().to_string());
            }
        }
        None
    }
}
