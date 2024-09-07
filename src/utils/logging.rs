use super::structs::Result;
use chrono::Local;
use fern::colors::{Color, ColoredLevelConfig};
use fern::{log_file, Dispatch, FormatCallback};
use log::{LevelFilter, Record};
use serde::Serialize;
use std::fmt::Arguments;
use std::path::Path;

#[cfg(debug_assertions)]
#[derive(Serialize)]
struct LogRecord<'a> {
    timestamp: String,
    level: String,
    target: &'a str,
    line: Option<u32>,
    message: String,
}

#[cfg(not(debug_assertions))]
#[derive(Serialize)]
struct LogRecord {
    timestamp: String,
    level: String,
    message: String,
}

#[cfg(debug_assertions)]
fn json_format(out: FormatCallback, record: &log::Record) {
    let log_record = LogRecord {
        timestamp: Local::now().format("%+").to_string(),
        level: record.level().to_string(),
        target: record.target(),
        line: record.line(),
        message: record.args().to_string(),
    };

    let json = serde_json::to_string(&log_record).unwrap();
    out.finish(format_args!("{}", json))
}

#[cfg(not(debug_assertions))]
fn json_format_without_target(out: FormatCallback, record: &log::Record) {
    let log_record = LogRecord {
        timestamp: Local::now().format("%+").to_string(),
        level: record.level().to_string(),
        message: record.args().to_string(),
    };

    let json = serde_json::to_string(&log_record).unwrap();
    out.finish(format_args!("{}", json))
}

#[cfg(debug_assertions)]
fn default_format(out: FormatCallback, record: &log::Record, colors: Option<ColoredLevelConfig>) {
    out.finish(format_args!(
        "{} [{}:{}:{}] - {}",
        Local::now().format("%+"),
        match colors {
            Some(c) => c.color(record.level()).to_string(),
            None => record.level().to_string(),
        },
        record.target(),
        record.line().unwrap_or(0),
        record.args()
    ))
}

#[cfg(not(debug_assertions))]
fn default_format_without_target(
    out: FormatCallback,
    record: &log::Record,
    colors: Option<ColoredLevelConfig>,
) {
    out.finish(format_args!(
        "{} [{}] - {}",
        Local::now().format("%+"),
        match colors {
            Some(c) => c.color(record.level()).to_string(),
            None => record.level().to_string(),
        },
        record.args()
    ))
}

#[cfg(debug_assertions)]
fn make_formatter(
    use_colors: bool,
    use_json: bool,
) -> impl Fn(FormatCallback, &Arguments, &Record) {
    let colors = ColoredLevelConfig::new()
        // use builder methods
        .info(Color::Green)
        .debug(Color::Blue)
        .trace(Color::Magenta);

    move |out: FormatCallback, _: &Arguments, record: &Record| {
        if use_json {
            json_format(out, record)
        } else {
            default_format(out, record, if use_colors { Some(colors) } else { None })
        }
    }
}

#[cfg(not(debug_assertions))]
fn make_formatter(
    use_colors: bool,
    use_json: bool,
) -> impl Fn(FormatCallback, &Arguments, &Record) {
    let colors = ColoredLevelConfig::new()
        // use builder methods
        .info(Color::Green)
        .debug(Color::Blue)
        .trace(Color::Magenta);

    move |out: FormatCallback, _: &Arguments, record: &Record| {
        if use_json {
            json_format_without_target(out, record)
        } else {
            default_format_without_target(out, record, if use_colors { Some(colors) } else { None })
        }
    }
}

pub fn log_init(
    log_level: LevelFilter,
    use_json: bool,
    log_file_location: Option<&Path>,
) -> Result<()> {
    let stderr_dispatcher = Dispatch::new()
        .format(make_formatter(true, use_json))
        .level(log_level)
        .chain(std::io::stderr());

    match log_file_location {
        Some(lfl) => {
            let file_dispatcher = Dispatch::new()
                .format(make_formatter(false, use_json))
                .level(log_level)
                .chain(log_file(lfl)?);

            fern::Dispatch::new()
                .chain(stderr_dispatcher)
                .chain(file_dispatcher)
                .apply()?;
        }
        None => {
            fern::Dispatch::new().chain(stderr_dispatcher).apply()?;
        }
    }

    Ok(())
}
