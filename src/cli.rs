//! Command-line interface, defined with clap's builder pattern.

use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

/// Parsed command-line arguments.
#[derive(Debug, Clone)]
pub struct Cli {
    /// Path to the input match-report CSV file.
    pub input: PathBuf,
    /// Path to the output `.xlsx` file to create.
    pub output: PathBuf,
    /// Verbosity level (number of `-v` flags supplied).
    pub verbosity: u8,
}

/// Builds the clap [`Command`] describing the CLI.
fn command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        // Show the full help screen when invoked with no arguments at all,
        // instead of a terse "required arguments were not provided" error.
        .arg_required_else_help(true)
        // Provide a `help` subcommand (e.g. `match-report-analyzer help`) as an
        // alternative to `--help`. We register it ourselves and render help for
        // it in `parse`, so clap's automatic one is disabled to avoid a clash.
        .disable_help_subcommand(true)
        .subcommand_negates_reqs(true)
        .subcommand(Command::new("help").about("Print this help message"))
        .arg(
            Arg::new("input")
                .value_name("INPUT_CSV")
                .help("Path to the match-report CSV file to convert")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output")
                .value_name("OUTPUT_XLSX")
                .help("Path of the highlighted Excel (.xlsx) file to create")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Increase logging verbosity (-v for debug, -vv for trace)")
                .action(ArgAction::Count),
        )
}

impl Cli {
    /// Parses the process arguments, exiting the process on error, `--help`,
    /// the `help` subcommand, or when no arguments are supplied.
    pub fn parse() -> Self {
        let mut cmd = command();
        let matches = cmd.get_matches_mut();
        if matches.subcommand_matches("help").is_some() {
            // Mirror the behavior of `--help`: print help to stdout and exit
            // successfully. `cmd` is still usable thanks to `get_matches_mut`.
            cmd.print_help().expect("failed to write help to stdout");
            println!();
            std::process::exit(0);
        }
        Self::from_matches(matches)
    }

    /// Builds a [`Cli`] from already-parsed [`ArgMatches`].
    fn from_matches(matches: ArgMatches) -> Self {
        Cli {
            input: matches
                .get_one::<PathBuf>("input")
                .expect("input is required")
                .clone(),
            output: matches
                .get_one::<PathBuf>("output")
                .expect("output is required")
                .clone(),
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
    fn parses_required_positional_arguments() {
        let matches = command()
            .try_get_matches_from(["app", "in.csv", "out.xlsx"])
            .expect("valid args");
        let cli = Cli::from_matches(matches);
        assert_eq!(cli.input, PathBuf::from("in.csv"));
        assert_eq!(cli.output, PathBuf::from("out.xlsx"));
        assert_eq!(cli.verbosity, 0);
    }

    #[test]
    fn counts_verbosity_flags() {
        let matches = command()
            .try_get_matches_from(["app", "-vv", "in.csv", "out.xlsx"])
            .expect("valid args");
        assert_eq!(Cli::from_matches(matches).verbosity, 2);
    }

    #[test]
    fn missing_output_is_an_error() {
        let result = command().try_get_matches_from(["app", "in.csv"]);
        assert!(result.is_err());
    }

    #[test]
    fn help_subcommand_is_recognized_without_positional_args() {
        // `subcommand_negates_reqs` lets `help` parse despite the otherwise
        // required INPUT_CSV / OUTPUT_XLSX positionals.
        let matches = command()
            .try_get_matches_from(["app", "help"])
            .expect("help subcommand should parse");
        assert!(matches.subcommand_matches("help").is_some());
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
