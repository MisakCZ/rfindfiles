use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const DATE_FORMAT: &str = "YYYY-MM-DD";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub folders: Vec<PathBuf>,
    pub extensions: Vec<String>,
    pub date_from: Option<SystemTime>,
    pub date_to: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    HelpRequested,
    Message(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HelpRequested => write!(f, "help requested"),
            Self::Message(message) => write!(f, "{message}"),
        }
    }
}

pub fn parse_folders(raw: &str) -> Vec<PathBuf> {
    raw.replace('\n', ",")
        .split(',')
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                return None;
            }

            let expanded = expand_home(trimmed);
            if expanded.is_dir() {
                Some(expanded)
            } else {
                None
            }
        })
        .collect()
}

pub fn parse_extensions(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|item| {
            let stripped = item.trim().to_ascii_lowercase();
            if stripped.is_empty() {
                return None;
            }

            if stripped.starts_with('.') {
                Some(stripped)
            } else {
                Some(format!(".{stripped}"))
            }
        })
        .collect()
}

pub fn parse_date(raw: &str, end_of_day: bool) -> Result<SystemTime, CliError> {
    let trimmed = raw.trim();
    let (year, month, day) = parse_ymd(trimmed)?;
    let days = days_from_civil(year, month, day);
    let seconds = days
        .checked_mul(86_400)
        .ok_or_else(|| CliError::Message("Datum je mimo podporovaný rozsah.".to_string()))?;
    let seconds = if end_of_day {
        seconds
            .checked_add(86_399)
            .ok_or_else(|| CliError::Message("Datum je mimo podporovaný rozsah.".to_string()))?
    } else {
        seconds
    };

    unix_seconds_to_system_time(seconds)
}

pub fn find_files(config: &Config) -> Vec<PathBuf> {
    let mut results = Vec::new();

    for folder in &config.folders {
        visit_dir(folder, config, &mut results);
    }

    results.sort();
    results
}

fn expand_home(path: &str) -> PathBuf {
    if path == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(path));
    }

    PathBuf::from(path)
}

fn parse_ymd(raw: &str) -> Result<(i32, u32, u32), CliError> {
    let mut parts = raw.split('-');
    let year = parts
        .next()
        .ok_or_else(|| invalid_date_message(raw))?
        .parse::<i32>()
        .map_err(|_| invalid_date_message(raw))?;
    let month = parts
        .next()
        .ok_or_else(|| invalid_date_message(raw))?
        .parse::<u32>()
        .map_err(|_| invalid_date_message(raw))?;
    let day = parts
        .next()
        .ok_or_else(|| invalid_date_message(raw))?
        .parse::<u32>()
        .map_err(|_| invalid_date_message(raw))?;

    if parts.next().is_some() || month == 0 || month > 12 || day == 0 || day > 31 {
        return Err(invalid_date_message(raw));
    }

    let max_day = days_in_month(year, month);
    if day > max_day {
        return Err(invalid_date_message(raw));
    }

    Ok((year, month, day))
}

fn invalid_date_message(raw: &str) -> CliError {
    CliError::Message(format!(
        "Neplatné datum '{raw}', očekávaný formát je {DATE_FORMAT}."
    ))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn unix_seconds_to_system_time(seconds: i64) -> Result<SystemTime, CliError> {
    let magnitude = u64::try_from(seconds.abs())
        .map_err(|_| CliError::Message("Datum je mimo podporovaný rozsah.".to_string()))?;
    let duration = Duration::from_secs(magnitude);

    if seconds >= 0 {
        Ok(SystemTime::UNIX_EPOCH + duration)
    } else {
        SystemTime::UNIX_EPOCH
            .checked_sub(duration)
            .ok_or_else(|| CliError::Message("Datum je mimo podporovaný rozsah.".to_string()))
    }
}

fn visit_dir(folder: &Path, config: &Config, results: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            visit_dir(&path, config, results);
            continue;
        }

        if !file_type.is_file() || !matches_extension(&path, &config.extensions) {
            continue;
        }

        let modified_at = match entry.metadata().and_then(|metadata| metadata.modified()) {
            Ok(modified_at) => modified_at,
            Err(_) => continue,
        };

        if let Some(date_from) = config.date_from {
            if modified_at < date_from {
                continue;
            }
        }

        if let Some(date_to) = config.date_to {
            if modified_at > date_to {
                continue;
            }
        }

        results.push(path);
    }
}

fn matches_extension(path: &Path, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }

    let suffix = match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => format!(".{}", ext.to_ascii_lowercase()),
        None => return false,
    };

    extensions.iter().any(|extension| extension == &suffix)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extensions_adds_dot_and_normalizes_case() {
        assert_eq!(
            parse_extensions("PDF, txt,.Rs"),
            vec![".pdf", ".txt", ".rs"]
        );
    }

    #[test]
    fn parse_date_accepts_valid_date() {
        assert!(parse_date("2026-03-23", false).is_ok());
    }

    #[test]
    fn parse_date_rejects_invalid_date() {
        let error = parse_date("2026-02-30", false).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Neplatné datum '2026-02-30', očekávaný formát je YYYY-MM-DD."
        );
    }

    #[test]
    fn parse_folders_expands_home_and_filters_missing_paths() {
        let current = std::env::current_dir().expect("current dir");
        let home = std::env::var("HOME").expect("home");
        let parsed = parse_folders(&format!(
            "{},~/definitely-missing-path,{home}",
            current.display()
        ));

        assert!(parsed.contains(&current));
        assert!(parsed.iter().all(|path| path.is_dir()));
    }
}
