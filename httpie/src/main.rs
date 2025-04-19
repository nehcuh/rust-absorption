use std::collections::HashMap;

use anyhow::{Result as AnyResult, anyhow};
use clap::Parser;
use colored::Colorize;
use mime::{APPLICATION_JSON, Mime};
use reqwest::{
    Client, Response,
    header::{CONTENT_TYPE, HeaderMap, USER_AGENT},
};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    Get(Get),
    Post(Post),
}

#[derive(Parser)]
pub struct Get {
    #[arg(value_parser=parse_url)]
    url: String,
}

fn parse_url(s: &str) -> AnyResult<String> {
    let _ = s.parse::<reqwest::Url>()?;
    Ok(s.into())
}

#[derive(Parser)]
pub struct Post {
    #[arg(value_parser=parse_url)]
    url: String,
    #[arg(value_parser=parse_kv_pair)]
    body: Vec<KvPair>,
}

fn parse_kv_pair(s: &str) -> AnyResult<KvPair> {
    Ok(s.try_into()?)
}

#[derive(Clone)]
struct KvPair {
    k: String,
    v: String,
}

impl TryFrom<&str> for KvPair {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> AnyResult<Self> {
        let mut splitter = value.split("=");
        let err = || anyhow!("Failed to parse {}", value);
        Ok(Self {
            k: splitter.next().ok_or_else(err)?.to_string(),
            v: splitter.next().ok_or_else(err)?.to_string(),
        })
    }
}

fn print_status(resp: &Response) {
    let status = format!("{:?}: {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

fn print_headers(resp: &Response) {
    for (k, v) in resp.headers() {
        println!(
            "{}",
            format!("{}: {:?}", k.to_string(), v.to_str().unwrap()).green()
        );
    }
    println!("\n");
}

fn print_body(m: Option<Mime>, body: &str) {
    match m {
        Some(v) if v == APPLICATION_JSON => {
            println!(
                "{}",
                format!("{}", jsonxf::pretty_print(body).unwrap()).cyan()
            );
        }
        _ => println!("{}", body),
    }
}

async fn print_response(resp: Response) -> AnyResult<()> {
    print_status(&resp);
    print_headers(&resp);
    let m = get_content_type(&resp);
    let body = resp.text().await?;
    print_body(m, &body);
    Ok(())
}

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get(CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

async fn get(client: Client, args: &Get) -> AnyResult<()> {
    let resp = client.get(&args.url).send().await?;
    print_response(resp).await?;
    Ok(())
}

async fn post(client: Client, args: &Post) -> AnyResult<()> {
    let mut headers = HashMap::new();
    for pair in &args.body {
        headers.insert(&pair.k, &pair.v);
    }
    let resp = client.post(&args.url).json(&headers).send().await?;
    print_response(resp).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    let cli = Cli::parse();
    let mut headers = HeaderMap::new();
    headers.insert("X-POWERED-BY", "Rust Httpie".parse()?);
    headers.insert(USER_AGENT, "Rust Httpie".parse()?);
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;
    match cli.subcmd {
        SubCommand::Get(ref args) => get(client, args).await?,
        SubCommand::Post(ref args) => post(client, args).await?,
    }
    Ok(())
}
