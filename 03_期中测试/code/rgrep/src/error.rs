use thiserror::Error;

/// RGrep错误定义
#[derive(Error, Debug)]
pub enum GrepError {
    #[error("Glob pattern error")]
    GlobPatternError(#[from] glob::PatternError),

    #[error("regex pattern error")]
    RegexPatternError(#[from] regex::Error),

    #[error("I/O error")]
    IoError(#[from] std::io::Error),
}
