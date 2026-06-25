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
    /// Parses the process arguments, exiting the process on error or `--help`.
    pub fn parse() -> Self {
        Self::from_matches(command().get_matches())
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
}
