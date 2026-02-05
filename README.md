# auto-resume

An intelligent, AI-powered CLI tool that automatically generates tailored, ATS-optimized resumes for specific job postings. By analyzing your GitHub profile and comparing it against job requirements, auto-resume intelligently selects your most relevant projects and generates professional resume content in PDF format.

## Features

- **GitHub Profile Analysis**: Automatically fetches and analyzes your GitHub repositories, extracting metadata like stars, languages, README content, and commit history
- **Job Description Processing**: Scrapes job postings from URLs (LinkedIn, Indeed, etc.) or loads from local files
- **AI-Powered Repository Ranking**: Uses LLM to intelligently rank your projects by relevance to the job posting
- **Interactive Repository Selection**: Review ranked repositories and select which projects to highlight
- **ATS-Optimized Content Generation**: Generates skills, projects, and experience sections optimized for Applicant Tracking Systems
- **Professional PDF Output**: Compiles into a polished PDF resume using LaTeX
- **Multi-Language Support**: English and Portuguese resume templates
- **Interactive Editing**: Edit the LaTeX source before PDF compilation for final customization

## Quick Start

### Prerequisites

- Rust 1.75+ (for building from source)
- GitHub account with public repositories
- LLM API key (default: Google Gemini; supports custom endpoints)
- LaTeX distribution (via Tectonic, automatically handled)

### Installation

```bash
git clone https://github.com/yourusername/auto-resume.git
cd auto-resume
cargo build --release
```

The compiled binary will be at `target/release/auto-resume`.

### Configuration

1. Copy the configuration template:
   ```bash
   cp config.default.toml config.toml
   ```

2. Edit `config.toml` with your information:
   ```toml
   [config.resume]
   full_name = "Your Name"
   country = "Brazil"
   city = "São Paulo"
   email = "your.email@example.com"
   phone = "+55 11 98765-4321"
   linkedin = "https://linkedin.com/in/yourprofile"
   github = "https://github.com/yourusername"
   site = "https://yourportfolio.com"

   [config.github]
   username = "yourusername"
   token = "ghp_YOUR_GITHUB_TOKEN"  # Optional but recommended for rate limits

   [config.llm]
   api_key = "YOUR_GEMINI_API_KEY"
   model = "gemini-3-flash-preview"
   # Optional: custom endpoint
   # endpoint = "https://your-custom-llm.com/v1/chat"
   ```

3. Get required API keys:
   - **GitHub Token** (optional): [GitHub Settings → Developer Settings → Personal Access Tokens](https://github.com/settings/tokens)
   - **Gemini API Key** (required): [Google AI Studio](https://aistudio.google.com/app/apikey)

## Usage

### Basic Usage

Generate resume for a job posting from URL:
```bash
./auto-resume --job-url "https://linkedin.com/jobs/view/1234567890"
```

Generate resume from local job description file:
```bash
./auto-resume --job-file job_description.txt
```

Generate resume in English:
```bash
./auto-resume --job-url "https://..." --language en
```

### Advanced Options

```
Options:
  -c, --config <FILE>       Path to configuration file (default: config.toml)
  -j, --job-url <URL>       URL to job posting
  --job-file <FILE>         Path to job description file
  -l, --language <LANG>     Resume language: 'en' or 'pt' (default: pt)
  -o, --output <FILE>       Output PDF file path (default: resume.pdf)
  --latex                   Save intermediate LaTeX file for inspection
  -v, --verbosity           Increase log verbosity (can be used multiple times)
  -h, --help                Show help message
```

### Workflow Example

```bash
# 1. Generate resume with interactive repository selection
./auto-resume --job-url "https://linkedin.com/jobs/view/1234567890" \
  --language en \
  --output my_resume_acme_corp.pdf

# 2. You'll be prompted to:
#    - Review ranked repositories
#    - Select which projects to include (e.g., "1 3 5")
#    - (Optional) Add additional repositories not on GitHub
#    - (Optional) Edit the LaTeX source before compilation

# 3. Resume is generated as my_resume_acme_corp.pdf
```

## How It Works

### 1. Configuration Loading
Reads personal info, GitHub credentials, and LLM API key from `config.toml`.

### 2. GitHub Data Collection
Fetches your public repositories via GitHub API, extracting:
- Repository name, URL, description
- Stars, forks, language distribution
- README content (project context)
- Commit history (activity level)

### 3. Job Description Processing
- **From URL**: Fetches HTML and extracts text content
- **From File**: Reads local text file
- **Fallback**: Uses generic software engineer template if none provided

The LLM cleans and structures the job description to extract:
- Position title
- Company name
- Key requirements and technologies
- Nice-to-have skills

### 4. Repository Ranking
The LLM analyzes your repositories against job requirements and ranks them by relevance, generating reasoning for each ranking decision.

### 5. Interactive Selection
You review the ranked repositories with visual indicators (★ stars) and select which ones to highlight in your resume. You can also manually add repositories not detected on GitHub.

### 6. Resume Content Generation
The LLM generates resume content using:
- Your selected repositories (real projects with GitHub links)
- Job description keywords and requirements
- Additional context you provide (education, experience, skills)

Generates:
- **Skills**: Categorized technical skills matching job requirements
- **Projects**: Descriptions of your selected GitHub projects
- **Experience**: Professional roles and accomplishments
- **Education**: Academic background

Content is automatically optimized for:
- ATS keyword matching
- Specific technologies mentioned in job posting
- Quantifiable metrics and results
- Industry-standard terminology

### 7. LaTeX Assembly
Inserts generated content into the resume template with:
- Personal information and contact details
- Language-specific formatting (English/Portuguese)
- Proper LaTeX escaping to prevent compilation errors

### 8. Optional Editing
Before PDF compilation, you can review and edit the LaTeX source file in your preferred editor.

### 9. PDF Compilation
Compiles LaTeX to PDF using Tectonic, generating the final resume file.

## Architecture

### Project Structure

```
src/
├── main.rs              # Application orchestration and CLI entry point
├── chat/
│   ├── agent.rs         # LLM integration and resume generation prompts
│   └── system_prompt.txt # ATS optimization guidelines for LLM
├── scraper/
│   ├── github.rs        # GitHub API data collection
│   └── job.rs           # Job description fetching
├── latex/
│   ├── assembler.rs     # LaTeX template assembly
│   └── template.tex     # Resume template (bilingual)
├── models/
│   └── github.rs        # GitHub API response types
└── utils/
    ├── cli.rs           # Command-line argument parsing
    ├── config.rs        # Configuration file management
    ├── select_repos.rs  # Interactive repository selection UI
    ├── cache.rs         # HTTP response caching
    └── log.rs           # Logging setup
```

### Technology Stack

- **Language**: Rust 2024 Edition
- **LLM Framework**: `rig-core` (supports Google Gemini, OpenAI, custom endpoints)
- **Async Runtime**: `tokio`
- **HTTP Client**: `reqwest` with `backon` for automatic retries
- **LaTeX/PDF**: `tectonic`
- **CLI**: `clap` with derive macros
- **Serialization**: `serde`, `serde_json`, `toml`
- **Parallelization**: `rayon`

## Configuration Reference

### `config.toml` Sections

#### `[config.resume]`
Personal information displayed on resume:
- `full_name`: Your full name
- `country`: Country of residence
- `city`: City of residence
- `email`: Contact email
- `phone`: Phone number
- `linkedin`: LinkedIn profile URL
- `github`: GitHub profile URL
- `site`: Personal website/portfolio URL

Optional context sections (appended to auto-generated content):
- `education_context`: Additional education info not in resume
- `experience_context`: Additional professional experience
- `skills_context`: Additional skills or certifications

#### `[config.github]`
GitHub API configuration:
- `username`: Your GitHub username (required)
- `token`: GitHub Personal Access Token (optional, recommended for rate limits)

#### `[config.llm]`
LLM API configuration:
- `api_key`: LLM API key (required)
- `model`: Model name (default: `gemini-3-flash-preview`)
- `endpoint`: Custom LLM endpoint URL (optional)
- `max_retries`: Retry attempts for failed requests (default: 3)

## Resume Optimization

The tool generates ATS-optimized resumes following these principles:

1. **Keyword Matching**: Incorporates keywords from the job posting
2. **Specific Technologies**: Lists version numbers and specific tools used
3. **Quantified Results**: Includes metrics, percentages, and measurable impact
4. **Clear Structure**: Hierarchical formatting that ATS systems parse correctly
5. **Industry Terminology**: Uses standard resume vocabulary
6. **Action Verbs**: Starts accomplishments with strong action verbs (Architected, Led, Optimized, etc.)

## Development

### Building from Source

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Development Environment (Nix)

```bash
nix flake update
nix develop
```

### Debugging

Enable verbose output for troubleshooting:

```bash
./auto-resume --verbosity trace --job-url "https://..." 2>&1 | tee debug.log
```

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
