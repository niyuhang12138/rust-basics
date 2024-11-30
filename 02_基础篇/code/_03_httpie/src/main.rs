use colored::*;
use std::collections::HashMap;

use anyhow::anyhow;
use clap::{builder::Str, Parser};
use reqwest::Client;
use url::Url;

/// A naive httpie implementation with Rust, can you imagine how easy it is?
#[derive(Debug, Parser)]
#[command(name = "httpie", version, author, long_about = None)]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,
}

// 子命令分别对应不同的HTTP方法, 目前只支持get/post
#[derive(Debug, Parser)]
enum SubCommand {
    Get(Get),
    Post(Post),
}

#[derive(Debug, Parser)]
struct Get {
    #[arg(short, long, value_parser = validate_url)]
    url: String,
}

#[derive(Debug, Parser)]
struct Post {
    #[arg(short, long, value_parser = validate_url)]
    url: String,

    #[arg(short, long, value_parser = validate_body)]
    body: String,
}

fn validate_url(url: &str) -> anyhow::Result<String> {
    let u = Url::parse(url)?;
    Ok(u.into())
}

fn validate_body(body: &str) -> anyhow::Result<String> {
    Ok(body.into())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();

    println!("opts: {:?}", opts);

    let clint = Client::new();

    let result = match opts.subcmd {
        SubCommand::Get(args) => get(clint, &args).await?,
        SubCommand::Post(args) => post(clint, &args).await?,
    };

    Ok(result)
}

async fn get(client: Client, args: &Get) -> anyhow::Result<()> {
    let resp = client.get(&args.url).send().await?;
    println!("Get: {:?}", resp.text().await?);
    Ok(())
}

async fn post(client: Client, args: &Post) -> anyhow::Result<()> {
    let mut body = HashMap::new();
    body.insert("key".to_string(), &args.body);
    let resp = client.post(&args.url).json(&body).send().await?;
    println!("Post: {:?}", resp.text().await?);
    Ok(())
}
