use std::path::PathBuf;

use clap::Parser;
use log::LevelFilter;

#[derive(Parser, Debug)]
#[command(name = "auto-resume")]
#[command(about = "Generate resumes tailored to job postings using GitHub data and AI", long_about = None)]
pub struct Args {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE", default_value = "config.toml")]
    pub config: PathBuf,

    /// URL to the job posting (LinkedIn, Indeed, etc.)
    #[arg(short, long, value_name = "URL")]
    pub job_url: Option<String>,

    /// Path to file containing job description
    #[arg(long, value_name = "FILE")]
    pub job_file: Option<PathBuf>,

    /// Resume language: en (English) or pt (Portuguese)
    #[arg(short, long, value_name = "LANG", default_value = "pt")]
    pub language: String,

    /// Output PDF file path
    #[arg(short, long, value_name = "FILE", default_value = "resume.pdf")]
    pub output: PathBuf,

    /// Sets the logger's verbosity level
    #[arg(short, long, value_name = "VERBOSITY", default_value_t = LevelFilter::Info)]
    pub verbosity: LevelFilter,
}
