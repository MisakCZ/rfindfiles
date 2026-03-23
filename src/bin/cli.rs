use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process;

use rfindfiles::{CliError, Config, DATE_FORMAT, find_files, parse_date, parse_extensions};

fn main() {
    let args = env::args_os().skip(1).collect::<Vec<_>>();

    match parse_cli(args) {
        Ok(config) => {
            let results = find_files(&config);
            for path in results {
                println!("{}", path.display());
            }
        }
        Err(CliError::HelpRequested) => {
            print_help();
        }
        Err(err) => {
            eprintln!("Chyba: {err}");
            eprintln!();
            print_help();
            process::exit(2);
        }
    }
}

fn parse_cli(args: Vec<OsString>) -> Result<Config, CliError> {
    let mut folders = Vec::new();
    let mut extensions = Vec::new();
    let mut date_from = None;
    let mut date_to = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        let raw = arg.to_string_lossy();
        match raw.as_ref() {
            "-h" | "--help" => return Err(CliError::HelpRequested),
            "--ext" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::Message("Chybí hodnota za --ext".to_string()))?;
                extensions.extend(parse_extensions(&value.to_string_lossy()));
            }
            "--date-from" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::Message("Chybí hodnota za --date-from".to_string()))?;
                date_from = Some(parse_date(&value.to_string_lossy(), false)?);
            }
            "--date-to" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::Message("Chybí hodnota za --date-to".to_string()))?;
                date_to = Some(parse_date(&value.to_string_lossy(), true)?);
            }
            value if value.starts_with("--") => {
                return Err(CliError::Message(format!("Neznámý přepínač: {value}")));
            }
            _ => {
                let path = PathBuf::from(arg);
                if !path.is_dir() {
                    return Err(CliError::Message(format!(
                        "Složka neexistuje nebo není adresář: {}",
                        path.display()
                    )));
                }
                folders.push(path);
            }
        }
    }

    if folders.is_empty() {
        return Err(CliError::Message(
            "Musíš zadat alespoň jednu složku.".to_string(),
        ));
    }

    Ok(Config {
        folders,
        extensions,
        date_from,
        date_to,
    })
}

fn print_help() {
    println!(
        "\
Použití:
  rfindfiles-cli [VOLBY] <SLOZKA> [DALSI_SLOZKA...]

Volby:
  --ext pdf,txt,rs       Filtr podle přípon
  --date-from YYYY-MM-DD Zahrnout soubory od tohoto data
  --date-to YYYY-MM-DD   Zahrnout soubory do tohoto data
  -h, --help             Zobrazit nápovědu

Příklady:
  rfindfiles-cli .
  rfindfiles-cli --ext rs,md .
  rfindfiles-cli --ext pdf --date-from 2026-01-01 ~/Dokumenty

Formát data:
  {DATE_FORMAT}
"
    );
}
