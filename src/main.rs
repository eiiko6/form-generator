use anyhow::Context;
use clap::Parser;

use form_generator::config::load_config;
use form_generator::run_server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config_path: String,

    #[arg(short, long, default_value = "answers.json")]
    output_file: String,

    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).ok();

    let cfg = load_config(&cli.config_path).context(format!("loading {}", cli.config_path))?;
    tracing::info!(
        "Loaded config: '{}', writing answers to '{}' with {} fields",
        cli.config_path,
        cli.output_file,
        cfg.fields.len()
    );

    let port = std::env::var("SERVER_PORT").unwrap_or("8081".to_string());
    let addr = format!("127.0.0.1:{port}");

    run_server(cfg, cli.output_file, &addr).await?;

    Ok(())
}
