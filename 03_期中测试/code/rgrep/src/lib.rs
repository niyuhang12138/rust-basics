use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Stdout},
    ops::Range,
    path::Path,
};

use clap::Parser;

mod error;
use colored::Colorize;
pub use error::GrepError;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;

/// 定义类型 在使用是可以简化复杂的类型的书写
type StrategyFn<W, R> = fn(&Path, BufReader<R>, &Regex, &mut W) -> Result<(), GrepError>;

/// 简化版本的grep, 支持正则表达式和文件通配符
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]

pub struct GrepConfig {
    // 用于查找文件的正则表达式
    #[arg(short, long)]
    pattern: String,
    // 文件通配符
    #[arg(short, long)]
    glob: String,
}

impl GrepConfig {
    // 使用缺省策略来查找匹配
    pub fn match_with_default_strategy(&self) -> Result<(), GrepError> {
        self.match_with(default_strategy)
    }

    // 是否某个策略来查找匹配
    pub fn match_with(&self, strategy: StrategyFn<Stdout, File>) -> Result<(), GrepError> {
        // 正则表达式
        let regex = Regex::new(&self.pattern)?;

        // 生成所有符合通配符的文件 glob::glob返回一个Result OK中包含一个Paths, 它实现了Iterator的迭代器
        let files = glob::glob(&self.glob)?.collect::<Vec<_>>();

        // 并行处理所有的文件
        files.into_par_iter().for_each(|v| {
            if let Ok(filename) = v {
                if let Ok(file) = File::open(&filename) {
                    let reader = BufReader::new(file);
                    let mut stdout = io::stdout();

                    if let Err(e) = strategy(filename.as_path(), reader, &regex, &mut stdout) {
                        println!("Internal error: {e:?}");
                    }
                }
            }
        });
        Ok(())
    }
}

/// 缺省策略, 从头到尾行查找, 最后输出到writer
pub fn default_strategy<W: io::Write, R: Read>(
    path: &Path,
    reader: BufReader<R>,
    pattern: &Regex,
    writer: &mut W,
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
        lineno.to_string().blue(),
        (prefix.chars().count() + 1).to_string().cyan(),
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
        let result = format_line("Hello, Try~", 1000, 7..10);
        let expected = format!(
            "{0: >6}:{1: <3} Hello, {2}~",
            "1000".blue(),
            "8".cyan(),
            "Try".red()
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn default_should_work() {
        let path = Path::new("test_file.txt");
        let reader = BufReader::new(File::open(&path).unwrap());
        let pattern = Regex::new(r"Hello World;").unwrap();
        let mut writer = Vec::new();
        default_strategy(path, reader, &pattern, &mut writer);
        let result = String::from_utf8(writer).unwrap();
        let expected = [
            String::from("test_file.txt"),
            format_line("Hello World;", 1, 0..12),
            format_line("Hello World;", 3, 0..12),
        ];
        println!("{result}");
        println!("{}", expected.join("\n"));
        assert_eq!(result, expected.join("\n") + "\n");
    }
}
