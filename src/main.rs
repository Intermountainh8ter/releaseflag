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
    /// Sweep removal of gate verdict files under error/hold/block
    /// missing-gate policies and write the results JSON.
    MissingSweep {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        out: PathBuf,
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
        Cmd::MissingSweep { config, out } => {
            let started = std::time::Instant::now();
            let started_at = chrono_free_now();
            let text = std::fs::read_to_string(&config)?;
            let cfg: ReleaseConfig = serde_json::from_str(&text)?;
            let base = config.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
            let sweep = releaseflag::missing::missing_sweep(&cfg, &base)
                .map_err(|e| anyhow::anyhow!(e))?;
            let doc = serde_json::json!({
                "missing_sweep": sweep,
                "provenance": {
                    "started_at": started_at,
                    "seeds": [0],
                    "command": format!("releaseflag missing-sweep --config {} --out {}", config.display(), out.display()),
                    "hardware": "cpu",
                    "wall_s": started.elapsed().as_secs_f64(),
                    "data_sources": [{"url": "projects/ai/releaseflag/examples/release-check/release-config.json", "license": "MIT (in-repo committed fixtures)"}]
                }
            });
            if let Some(d) = out.parent() {
                std::fs::create_dir_all(d)?;
            }
            std::fs::write(&out, serde_json::to_string_pretty(&doc)? + "\n")?;
        }
        Cmd::Version => println!("releaseflag {}", env!("CARGO_PKG_VERSION")),
    }
    Ok(())
}

/// UTC ISO-8601 timestamp from the system clock (no extra dependency).
fn chrono_free_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}
