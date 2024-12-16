use anyhow::Result;
use clap::Parser;
use grep::{GrepConfig, GrepError};
use tracing::info;

fn main() -> Result<(), GrepError> {
    tracing_subscriber::fmt::init();
    info!("Grep utils start...");

    // 初始化命令行参数
    let grep_config = GrepConfig::parse();

    // info!("params take in ok: {grep_config:?}");

    info!("Matches the content list");
    grep_config.match_with_default_strategy()?;

    Ok(())
}
