//! Command-line entry point for the match-report-analyzer.

use std::path::Path;
use std::process::ExitCode;

use tracing::{error, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use match_report_analyzer::cli::{Cli, Cmd};

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

    match cli.command {
        Cmd::Xlsx { input, output } => run_xlsx(&input, &output),
        Cmd::Graph { input, output } => run_graph(&input, &output),
        Cmd::Grid { input, output } => run_grid(&input, &output),
    }
}

/// Corrects the output path's extension for the format being written, warning
/// when an adjustment was needed (e.g. the legacy `.xls` for the Excel output),
/// so the reading application opens the file cleanly.
fn normalize_output(output: &Path, extension: &str) -> std::path::PathBuf {
    let normalized = match_report_analyzer::normalize_output_path(output, extension);
    if normalized != output {
        warn!(
            requested = %output.display(),
            writing = %normalized.display(),
            "adjusted output extension to match the format being written"
        );
    }
    normalized
}

/// Runs the `xlsx` subcommand: converts the match-report CSV at `input` into a
/// highlighted Excel workbook at `output`.
fn run_xlsx(input: &Path, output: &Path) -> ExitCode {
    let output = normalize_output(output, "xlsx");
    match match_report_analyzer::convert_to_xlsx(input, &output) {
        Ok(stats) => {
            println!(
                "Wrote {} ({} rows, {} pairs; {} matching, {} differing, {} missing cells highlighted)",
                output.display(),
                stats.rows,
                stats.pairs,
                stats.matching,
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

/// Runs the `graph` subcommand: analyzes the match-report CSV at `input` into
/// an interactive similarity-graph document at `output`.
fn run_graph(input: &Path, output: &Path) -> ExitCode {
    let output = normalize_output(output, "html");
    match match_report_analyzer::convert_to_graph(input, &output) {
        Ok(stats) => {
            println!(
                "Wrote {} ({} assets, {} matches, {} clusters)",
                output.display(),
                stats.nodes,
                stats.edges,
                stats.clusters
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            error!("{err}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the `grid` subcommand: renders the match-report CSV at `input` as an
/// interactive data-grid document at `output`.
fn run_grid(input: &Path, output: &Path) -> ExitCode {
    let output = normalize_output(output, "html");
    match match_report_analyzer::convert_to_grid(input, &output) {
        Ok(stats) => {
            println!(
                "Wrote {} ({} rows, {} columns, {} pairs)",
                output.display(),
                stats.rows,
                stats.columns,
                stats.pairs
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            error!("{err}");
            ExitCode::FAILURE
        }
    }
}
