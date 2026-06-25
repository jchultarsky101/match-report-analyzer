//! Command-line entry point for the match-report-analyzer.

use std::process::ExitCode;

use tracing::{error, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use match_report_analyzer::cli::Cli;

/// Initializes the tracing subscriber.
///
/// The log level is taken from the `RUST_LOG` environment variable when set,
/// otherwise it is derived from the `-v` verbosity flags.
fn init_tracing(verbosity: u8) {
    let default_level = match verbosity {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbosity);

    // Excel only opens this format cleanly from a `.xlsx` file, so correct the
    // extension if the user supplied something else (e.g. the legacy `.xls`).
    let output = match_report_analyzer::normalize_output_path(&cli.output);
    if output != cli.output {
        warn!(
            requested = %cli.output.display(),
            writing = %output.display(),
            "adjusted output extension to .xlsx so Excel can open the file"
        );
    }

    match match_report_analyzer::convert(&cli.input, &output) {
        Ok(stats) => {
            println!(
                "Wrote {} ({} rows, {} pairs; {} differing, {} missing cells highlighted)",
                output.display(),
                stats.rows,
                stats.pairs,
                stats.different,
                stats.missing
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            error!("{err}");
            ExitCode::FAILURE
        }
    }
}
