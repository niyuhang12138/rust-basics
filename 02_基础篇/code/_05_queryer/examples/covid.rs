use anyhow::Result;
use polars::prelude::*;
use std::io::Cursor;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let url = "https://raw.githubusercontent.com/owid/covid-19-data/master/public/data/latest/owid-covid-latest.csv";
    let data = reqwest::get(url).await?.text().await?;

    // 使用polars直接请求
    let df = CsvReader::new(Cursor::new(data)).finish()?;

    // let filtered = df.filter(&df["new_deaths"].gt(500)?);

    Ok(())
}
