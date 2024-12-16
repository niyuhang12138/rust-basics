use std::{
    fmt::Write,
    fs::File,
    io::{self, BufRead, BufReader, Read, Stdout},
    ops::Range,
    path::Path,
};

use colored::Colorize;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;

use anyhow::Result;

use clap::Parser;

mod error;

pub use error::GrepError;

pub type StrategyFn =
    fn(&Path, &Regex, &mut dyn BufRead, &mut dyn io::Write) -> Result<(), GrepError>;

/// `Grep` file search content util
#[derive(Debug, Parser)]
#[command(version, about = "`Grep` file search content util", long_about = None)]
pub struct GrepConfig {
    #[arg(short, long)]
    pattern: String,

    #[arg(short, long)]
    glob: String,
}

impl GrepConfig {
    pub fn match_with_default_strategy(&self) -> Result<(), GrepError> {
        self.match_with(default_strategy)
    }

    pub fn match_with(&self, strategy_fn: StrategyFn) -> Result<(), GrepError> {
        let regex = Regex::new(&self.pattern)?;
        let files = glob::glob(&self.glob)?.collect::<Vec<_>>();

        files.into_par_iter().for_each(|v| {
            if let Ok(filepath) = v {
                if let Ok(file) = File::open(&filepath) {
                    let mut reader = BufReader::new(file);
                    let mut stdout = io::stdout();
                    if let Err(e) =
                        strategy_fn(filepath.as_path(), &regex, &mut reader, &mut stdout)
                    {
                        println!("Internal error: {e:?}");
                    }
                }
            }
        });
        Ok(())
    }
}

pub fn default_strategy(
    path: &Path,
    pattern: &Regex,
    reader: &mut dyn BufRead,
    writer: &mut dyn io::Write,
) -> Result<(), GrepError> {
    let matches = reader
        .lines()
        .enumerate()
        .map(|(lineno, line)| {
            line.ok()
                .map(|line| {
                    pattern
                        .find(&line)
                        .map(|m| format_line(&line, lineno + 1, m.range()))
                })
                .flatten()
        })
        .filter_map(|v| v.ok_or(()).ok())
        .join("\n");

    if !matches.is_empty() {
        writer.write(path.display().to_string().green().as_bytes())?;
        writer.write(b"\n")?;
        writer.write(matches.as_bytes())?;
        writer.write(b"\n")?;
    }

    Ok(())
}

pub fn format_line(line: &str, lineno: usize, range: Range<usize>) -> String {
    let Range { start, end } = range;
    let prefix = &line[..start];
    format!(
        "{0: >6}:{1: <3} {2}{3}{4}",
        (lineno).to_string().blue(),
        start.to_string().cyan(),
        prefix,
        &line[start..end].red(),
        &line[end..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_line_should_work() {
        let result = format_line("Hello, Tyr~", 1000, 7..10);
        let expected = format!(
            "{0: >6}:{1: <3} Hello, {2}~",
            "1000".blue(),
            "7".cyan(),
            "Tyr".red()
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn default_strategy_should_work() {
        let path = Path::new("src/main.rs");
        let input = b"hello world!\nhey Tyr!";
        let mut reader = BufReader::new(&input[..]);
        let pattern = Regex::new(r"he\w+").unwrap();
        let mut writer = Vec::new();
        default_strategy(path, &pattern, &mut reader, &mut writer).unwrap();
        let result = String::from_utf8(writer).unwrap();
        let expected = [
            String::from("src/main.rs"),
            format_line("hello world!", 1, 0..5),
            format_line("hey Tyr!\n", 2, 0..3),
        ];

        assert_eq!(result, expected.join("\n"));
    }
}
