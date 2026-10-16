//! releaseflag CLI.
//!
//! `decide --config release.json` aggregates upstream verdicts and
//! prints the Report JSON. Exit code: 0 Pass, 1 Hold, 2 Block.

use clap::{Parser, Subcommand};
use releaseflag::{aggregate, Decision, ReleaseConfig};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "releaseflag", version, about = "Release-time aggregator across the bench suite")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Aggregate upstream verdicts and print the Report. Exit 0/1/2
    /// for Pass/Hold/Block.
    Decide {
        /// Path to the release config JSON.
        #[arg(long)]
        config: PathBuf,
    },
    /// Print the version banner and exit.
    Version,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Decide { config } => {
            let text = std::fs::read_to_string(&config)?;
            let cfg: ReleaseConfig = serde_json::from_str(&text)?;
            let base_dir = config
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            match aggregate(&cfg, &base_dir) {
                Ok(report) => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                    match report.decision {
                        Decision::Pass => {}
                        Decision::Hold => std::process::exit(1),
                        Decision::Block => std::process::exit(2),
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(3);
                }
            }
        }
        Cmd::Version => println!("releaseflag {}", env!("CARGO_PKG_VERSION")),
    }
    Ok(())
}
