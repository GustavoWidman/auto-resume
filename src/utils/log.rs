use std::io::{Error, Write};

use colog::format::CologStyle;
use colog::formatter;
use colored::Colorize;
use env_logger::fmt::Formatter;
use env_logger::{Builder, Target, WriteStyle};
use log::{Level, LevelFilter, Record};

struct CustomLevelTokens;

impl CologStyle for CustomLevelTokens {
    fn level_token(&self, level: &Level) -> &str {
        match *level {
            Level::Error => "ERROR",
            Level::Warn => "WARN ",
            Level::Info => "INFO ",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    fn level_color(&self, level: &Level, msg: &str) -> String {
        match level {
            Level::Error => msg.red().bold(),
            Level::Warn => msg.yellow().bold(),
            Level::Info => msg.green().bold(),
            Level::Debug => msg.cyan(),
            Level::Trace => msg.magenta(),
        }
        .to_string()
    }

    fn prefix_token(&self, level: &Level) -> String {
        format!(
            "{} {}",
            chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S.%6f")
                .to_string()
                .white(),
            self.level_color(level, self.level_token(level)),
        )
    }

    fn format(&self, buf: &mut Formatter, record: &Record<'_>) -> Result<(), Error> {
        let sep = self.line_separator();
        let prefix = self.prefix_token(&record.level());

        let string = match &record.level() {
            Level::Error => record
                .args()
                .to_string()
                .replace('\n', &sep)
                .bold()
                .to_string(),
            Level::Warn | Level::Info => record.args().to_string().replace('\n', &sep),
            Level::Debug | Level::Trace => record
                .args()
                .to_string()
                .replace('\n', &sep)
                .white()
                .to_string(),
        };

        writeln!(buf, "{} {}", prefix, string)
    }
}

pub struct Logger;

impl Logger {
    pub fn init(level: LevelFilter) {
        Builder::new()
            .filter("auto_resume".into(), level)
            .target(Target::Stdout)
            .format(formatter(CustomLevelTokens))
            .write_style(WriteStyle::Always)
            .init();
    }
}
