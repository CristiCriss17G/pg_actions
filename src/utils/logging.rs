use chrono::Local;
use colored::*;
use env_logger::fmt::Formatter;
use env_logger::{Builder, Env};
use log::{Level, LevelFilter};
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct LogRecord<'a> {
    timestamp: String,
    level: String,
    target: Option<&'a str>,
    line: Option<u32>,
    message: String,
}

fn colorize_level(level: Level) -> String {
    match level {
        Level::Error => "ERROR".red().to_string(),
        Level::Warn => "WARN".yellow().to_string(),
        Level::Info => "INFO".green().to_string(),
        Level::Debug => "DEBUG".blue().to_string(),
        Level::Trace => "TRACE".magenta().to_string(),
    }
}

fn json_format(buf: &mut Formatter, record: &log::Record) -> std::io::Result<()> {
    let log_record = LogRecord {
        timestamp: Local::now().format("%+").to_string(),
        level: record.level().to_string(),
        target: Some(record.target()),
        line: record.line(),
        message: record.args().to_string(),
    };

    let json = serde_json::to_string(&log_record).unwrap();
    writeln!(buf, "{}", json)
}

fn json_format_without_target(buf: &mut Formatter, record: &log::Record) -> std::io::Result<()> {
    let log_record = LogRecord {
        timestamp: Local::now().format("%+").to_string(),
        level: record.level().to_string(),
        target: None,
        line: None,
        message: record.args().to_string(),
    };

    let json = serde_json::to_string(&log_record).unwrap();
    writeln!(buf, "{}", json)
}

fn default_format(buf: &mut Formatter, record: &log::Record) -> std::io::Result<()> {
    writeln!(
        buf,
        "{} [{}:{}:{}] - {}",
        Local::now().format("%+"),
        colorize_level(record.level()),
        record.target(),
        record.line().unwrap_or(0),
        record.args()
    )
}

fn default_format_without_target(buf: &mut Formatter, record: &log::Record) -> std::io::Result<()> {
    writeln!(
        buf,
        "{} [{}] - {}",
        Local::now().format("%+"),
        colorize_level(record.level()),
        record.args()
    )
}

pub fn log_init(log_level: LevelFilter, use_json: bool) {
    if cfg!(debug_assertions) {
        Builder::from_env(Env::default())
            .format(if use_json {
                json_format
            } else {
                default_format
            })
            .filter_level(log_level)
            .init();
    } else {
        Builder::from_env(Env::default())
            .format(if use_json {
                json_format_without_target
            } else {
                default_format_without_target
            })
            .filter_level(log_level)
            .init();
    }
}
