use crate::utils::config::{Config, ResumeItem};
use crate::utils::misc::strip_url;

static TEMPLATE: &str = include_str!("template.tex");
static LOCALE_MAP_EN: [(&str, &str); 4] = [
    ("EDUCATION_HEADER", "Education"),
    ("SKILLS_HEADER", "Technical Skills"),
    ("EXPERIENCE_HEADER", "Professional Experience"),
    ("PROJECTS_HEADER", "Key Projects"),
];
static LOCALE_MAP_PT: [(&str, &str); 4] = [
    ("EDUCATION_HEADER", "Educação"),
    ("SKILLS_HEADER", "Habilidades Técnicas"),
    ("EXPERIENCE_HEADER", "Experiência Profissional"),
    ("PROJECTS_HEADER", "Projetos e Performance"),
];

#[derive(Debug, Clone, Default)]
pub enum ResumeLanguage {
    #[default]
    English,
    Portuguese,
}

impl From<&str> for ResumeLanguage {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pt" | "pt-br" | "portuguese" => ResumeLanguage::Portuguese,
            "en" | "en-us" | "english" => ResumeLanguage::English,
            _ => ResumeLanguage::default(),
        }
    }
}

pub struct LatexResumeAssembler {
    language: ResumeLanguage,
    config: Config,
}

impl LatexResumeAssembler {
    pub fn new(config: Config, language: impl Into<Option<ResumeLanguage>>) -> Self {
        Self {
            language: language.into().unwrap_or_default(),
            config,
        }
    }

    pub fn assemble(&self) -> String {
        Self::localize(TEMPLATE.to_string(), &self.language)
            .replace(
                "<<NAME>>",
                &Self::escape_latex(&self.config.resume.full_name),
            )
            .replace("<<CITY>>", &Self::escape_latex(&self.config.resume.city))
            .replace(
                "<<COUNTRY>>",
                &Self::escape_latex(&self.config.resume.country),
            )
            .replace("<<HEADER>>", &self.header())
            .replace("<<EDUCATION>>", &Self::items(&self.config.resume.education))
            .replace("<<SKILLS>>", &Self::items(&self.config.resume.skills))
            .replace(
                "<<EXPERIENCE>>",
                &Self::items(&self.config.resume.experience),
            )
            .replace("<<PROJECTS>>", &Self::items(&self.config.resume.projects))
    }

    fn header(&self) -> String {
        let mut header = String::new();

        if let Some(email) = &self.config.resume.email {
            header.push_str(&format!(
                "\\ $|$ \\ \\href{{mailto:{}}}{{{}}} ",
                email, email
            ));
        }

        if let Some(phone) = &self.config.resume.phone {
            header.push_str(&format!("\\ $|$ \\ {}", phone));
        }

        if let Some(linkedin) = &self.config.resume.linkedin {
            header.push_str(&format!(
                "\\ $|$ \\ \\href{{{}}}{{{}}} ",
                linkedin,
                strip_url(linkedin)
            ));
        }

        if let Some(github) = &self.config.resume.github {
            header.push_str(&format!(
                "\\ $|$ \\ \\href{{{}}}{{{}}} ",
                github,
                strip_url(github)
            ));
        }

        if let Some(site) = &self.config.resume.site {
            header.push_str(&format!(
                "\\ $|$ \\ \\href{{{}}}{{{}}} ",
                site,
                strip_url(site)
            ));
        }

        header
    }

    fn item(item: &ResumeItem) -> String {
        let mut out = String::new();

        if let Some(title) = &item.title {
            let mut title = format!("\\noindent \\textbf{{{}}}", Self::escape_latex(title));

            if let Some(location) = &item.location {
                match &item.link {
                    Some(link) => {
                        title.push_str(&format!(
                            " \\hfill \\href{{{}}}{{{}}}",
                            link,
                            Self::escape_latex(location)
                        ));
                    }
                    None => {
                        title.push_str(&format!(" \\hfill {}", Self::escape_latex(location)));
                    }
                }
            }

            if item.description.is_some() {
                title.push_str(" \\\\");
            }

            out.push_str(&title);
            out.push('\n');
        }

        if let Some(description) = &item.description {
            let mut description = format!("\\textit{{{}}}", Self::escape_latex(description));

            if let Some(date) = &item.date {
                description.push_str(&format!(" \\hfill {} ", date));
            }

            out.push_str(&description);
            out.push('\n');
        }

        out.push_str("\\begin{itemize}[noitemsep,topsep=0pt,leftmargin=*]\n");
        for bullet in &item.items {
            out.push_str(&format!("    \\item {}\n", Self::escape_latex(bullet)));
        }
        out.push_str("\\end{itemize}\n");

        out
    }

    fn items(items: &[ResumeItem]) -> String {
        items
            .iter()
            .map(Self::item)
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn escape_latex(text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '*' && chars.peek() == Some(&'*') {
                chars.next();
                let mut bold_text = String::new();
                let mut found_end = false;

                while let Some(bc) = chars.next() {
                    if bc == '*' && chars.peek() == Some(&'*') {
                        chars.next();
                        found_end = true;
                        break;
                    }
                    bold_text.push(bc);
                }

                if found_end {
                    result.push_str(&format!("\\textbf{{{}}}", Self::escape_latex(&bold_text)));
                } else {
                    result.push_str("**");
                    result.push_str(&Self::escape_latex(&bold_text));
                }
            } else if c == '`' {
                let mut code_text = String::new();
                let mut found_end = false;

                for bc in chars.by_ref() {
                    if bc == '`' {
                        found_end = true;
                        break;
                    }
                    code_text.push(bc);
                }

                if found_end {
                    result.push_str(&format!("\\texttt{{{}}}", Self::escape_latex(&code_text)));
                } else {
                    result.push('`');
                    result.push_str(&Self::escape_latex(&code_text));
                }
            } else {
                result.push_str(&match c {
                    '&' => "\\&".to_string(),
                    '%' => "\\%".to_string(),
                    '$' => "\\$".to_string(),
                    '#' => "\\#".to_string(),
                    '_' => "\\_".to_string(),
                    '{' => "\\{".to_string(),
                    '}' => "\\}".to_string(),
                    '^' => "\\textasciicircum{}".to_string(),
                    '~' => "\\textasciitilde{}".to_string(),
                    '\\' => "\\textbackslash{}".to_string(),
                    _ => c.to_string(),
                });
            }
        }

        result
    }

    fn localize(mut template: String, language: &ResumeLanguage) -> String {
        let locale_map = match language {
            ResumeLanguage::English => &LOCALE_MAP_EN,
            ResumeLanguage::Portuguese => &LOCALE_MAP_PT,
        };

        for (key, value) in locale_map.iter() {
            template = template.replace(&format!("<<{}>>", key), value);
        }

        template
    }
}
