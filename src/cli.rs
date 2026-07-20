//! Command-line interface, defined with clap's builder pattern.
//!
//! The tool is organized as subcommands, one per generated file type. The
//! first of these is [`xlsx`](Cmd::Xlsx), which converts a match-report CSV
//! into a color-highlighted Excel workbook.

use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

/// Name of the subcommand that generates an Excel workbook.
const XLSX_SUBCOMMAND: &str = "xlsx";

/// Name of the subcommand that generates the interactive similarity graph.
const GRAPH_SUBCOMMAND: &str = "graph";

/// Former name of the graph subcommand, kept as a hidden alias.
const GRAPH_SUBCOMMAND_ALIAS: &str = "html";

/// Name of the subcommand that generates the interactive data grid.
const GRID_SUBCOMMAND: &str = "grid";

/// Parsed command-line arguments: global options plus the selected subcommand.
#[derive(Debug, Clone)]
pub struct Cli {
    /// The subcommand to run.
    pub command: Cmd,
    /// Verbosity level (number of `-v` flags supplied).
    pub verbosity: u8,
}

/// The available subcommands, one per generated file type.
#[derive(Debug, Clone)]
pub enum Cmd {
    /// Convert a match-report CSV into a color-highlighted Excel workbook.
    Xlsx {
        /// Path to the input match-report CSV file.
        input: PathBuf,
        /// Path to the output `.xlsx` file to create.
        output: PathBuf,
    },
    /// Analyze a match-report CSV into an interactive HTML similarity graph.
    Graph {
        /// Path to the input match-report CSV file.
        input: PathBuf,
        /// Path to the output `.html` file to create.
        output: PathBuf,
    },
    /// Render a match-report CSV as an interactive HTML data grid.
    Grid {
        /// Path to the input match-report CSV file.
        input: PathBuf,
        /// Path to the output `.html` file to create.
        output: PathBuf,
    },
}

/// Builds the clap [`Command`] describing the CLI.
fn command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        // Show the full help screen when invoked with no arguments at all,
        // instead of a terse "missing subcommand" error. With subcommands
        // present, clap also provides the `help` subcommand automatically.
        .arg_required_else_help(true)
        .subcommand_required(true)
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Increase logging verbosity (-v for debug, -vv for trace)")
                .action(ArgAction::Count)
                .global(true),
        )
        .subcommand(
            Command::new(XLSX_SUBCOMMAND)
                .about("Convert a match-report CSV into a color-highlighted Excel (.xlsx) workbook")
                .arg(input_arg())
                .arg(
                    Arg::new("output")
                        .value_name("OUTPUT_XLSX")
                        .help("Path of the highlighted Excel (.xlsx) file to create")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                ),
        )
        .subcommand(
            Command::new(GRAPH_SUBCOMMAND)
                .alias(GRAPH_SUBCOMMAND_ALIAS)
                .about("Analyze a match-report CSV into an interactive HTML similarity graph")
                .arg(input_arg())
                .arg(
                    Arg::new("output")
                        .value_name("OUTPUT_HTML")
                        .help("Path of the interactive HTML graph file to create")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                ),
        )
        .subcommand(
            Command::new(GRID_SUBCOMMAND)
                .about(
                    "Render a match-report CSV as an interactive HTML data grid \
                     (search, sort, SQL-like filtering, what-if edits)",
                )
                .arg(input_arg())
                .arg(
                    Arg::new("output")
                        .value_name("OUTPUT_HTML")
                        .help("Path of the interactive HTML grid file to create")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                ),
        )
}

/// The input-CSV positional argument, shared by every subcommand.
fn input_arg() -> Arg {
    Arg::new("input")
        .value_name("INPUT_CSV")
        .help("Path to the match-report CSV file to analyze")
        .required(true)
        .value_parser(value_parser!(PathBuf))
}

impl Cli {
    /// Parses the process arguments, exiting the process on error, `--help`,
    /// the `help` subcommand, or when no arguments are supplied.
    pub fn parse() -> Self {
        Self::from_matches(command().get_matches())
    }

    /// Builds a [`Cli`] from already-parsed [`ArgMatches`].
    fn from_matches(matches: ArgMatches) -> Self {
        fn paths(sub: &ArgMatches) -> (PathBuf, PathBuf) {
            (
                sub.get_one::<PathBuf>("input")
                    .expect("input is required")
                    .clone(),
                sub.get_one::<PathBuf>("output")
                    .expect("output is required")
                    .clone(),
            )
        }
        let command = match matches.subcommand() {
            Some((XLSX_SUBCOMMAND, sub)) => {
                let (input, output) = paths(sub);
                Cmd::Xlsx { input, output }
            }
            Some((GRAPH_SUBCOMMAND, sub)) => {
                let (input, output) = paths(sub);
                Cmd::Graph { input, output }
            }
            Some((GRID_SUBCOMMAND, sub)) => {
                let (input, output) = paths(sub);
                Cmd::Grid { input, output }
            }
            _ => unreachable!("the parser requires a known subcommand"),
        };
        Cli {
            command,
            // `verbose` is global, so it is propagated to (and readable from)
            // the top-level matches regardless of where it appeared.
            verbosity: matches.get_count("verbose"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_command_definition() {
        // `debug_assert` validates the command graph for configuration mistakes.
        command().debug_assert();
    }

    #[test]
    fn parses_xlsx_subcommand_with_positional_arguments() {
        let matches = command()
            .try_get_matches_from(["app", "xlsx", "in.csv", "out.xlsx"])
            .expect("valid args");
        let cli = Cli::from_matches(matches);
        let Cmd::Xlsx { input, output } = cli.command else {
            panic!("expected the xlsx subcommand");
        };
        assert_eq!(input, PathBuf::from("in.csv"));
        assert_eq!(output, PathBuf::from("out.xlsx"));
        assert_eq!(cli.verbosity, 0);
    }

    #[test]
    fn parses_graph_subcommand_with_positional_arguments() {
        let matches = command()
            .try_get_matches_from(["app", "graph", "in.csv", "out.html"])
            .expect("valid args");
        let Cmd::Graph { input, output } = Cli::from_matches(matches).command else {
            panic!("expected the graph subcommand");
        };
        assert_eq!(input, PathBuf::from("in.csv"));
        assert_eq!(output, PathBuf::from("out.html"));
    }

    #[test]
    fn html_still_works_as_a_graph_alias() {
        let matches = command()
            .try_get_matches_from(["app", "html", "in.csv", "out.html"])
            .expect("valid args");
        assert!(matches!(
            Cli::from_matches(matches).command,
            Cmd::Graph { .. }
        ));
    }

    #[test]
    fn parses_grid_subcommand_with_positional_arguments() {
        let matches = command()
            .try_get_matches_from(["app", "grid", "in.csv", "out.html"])
            .expect("valid args");
        let Cmd::Grid { input, output } = Cli::from_matches(matches).command else {
            panic!("expected the grid subcommand");
        };
        assert_eq!(input, PathBuf::from("in.csv"));
        assert_eq!(output, PathBuf::from("out.html"));
    }

    #[test]
    fn counts_verbosity_flags_before_the_subcommand() {
        let matches = command()
            .try_get_matches_from(["app", "-vv", "xlsx", "in.csv", "out.xlsx"])
            .expect("valid args");
        assert_eq!(Cli::from_matches(matches).verbosity, 2);
    }

    #[test]
    fn counts_verbosity_flags_after_the_subcommand() {
        // `verbose` is a global flag, so it is accepted subcommand-side too.
        let matches = command()
            .try_get_matches_from(["app", "xlsx", "-v", "in.csv", "out.xlsx"])
            .expect("valid args");
        assert_eq!(Cli::from_matches(matches).verbosity, 1);
    }

    #[test]
    fn missing_output_is_an_error() {
        let result = command().try_get_matches_from(["app", "xlsx", "in.csv"]);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_subcommand_is_an_error() {
        let err = command()
            .try_get_matches_from(["app", "frobnicate"])
            .expect_err("unknown subcommand should be rejected");
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn help_subcommand_displays_help() {
        // With subcommands registered, clap provides `help` automatically.
        let err = command()
            .try_get_matches_from(["app", "help"])
            .expect_err("help subcommand should request help display");
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn no_arguments_triggers_help_display() {
        let err = command()
            .try_get_matches_from(["app"])
            .expect_err("no args should request help display");
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }
}
