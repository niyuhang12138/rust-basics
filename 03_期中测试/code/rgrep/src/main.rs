use anyhow::Result;
use clap::Parser;
use rgrep::GrepConfig;

fn main() -> Result<()> {
    let config = GrepConfig::parse();
    config.match_with_default_strategy()?;
    Ok(())
}
