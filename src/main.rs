// Don't spawn a console window alongside the GUI on Windows release builds.
// Kept in debug builds so `println!`/panics remain visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! match-report-analyzer — a cross-platform native desktop GUI for analyzing
//! Physna geometric match-report CSV exports.
//!
//! Built with [Iced](https://iced.rs/). This file wires up the top-level
//! application state, the message loop, and the root view. Domain logic
//! (CSV parsing, filtering, summarizing) will live in dedicated modules as the
//! app grows.

use iced::widget::{button, column, container, text};
use iced::{Element, Task, Theme};

/// Application entry point.
fn main() -> iced::Result {
    iced::application(
        MatchReportAnalyzer::TITLE,
        MatchReportAnalyzer::update,
        MatchReportAnalyzer::view,
    )
    .theme(|_| Theme::Dark)
    .run()
}

/// Top-level application state.
#[derive(Default)]
struct MatchReportAnalyzer {
    /// Path of the currently loaded match report, if any.
    loaded_report: Option<String>,
}

/// Messages produced by user interaction and background tasks.
#[derive(Debug, Clone)]
enum Message {
    /// The user pressed the "Open report…" button.
    OpenReportPressed,
}

impl MatchReportAnalyzer {
    /// Window title shown by the OS.
    const TITLE: &'static str = "Match Report Analyzer";

    /// Handle a [`Message`], updating state and optionally scheduling async work.
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenReportPressed => {
                // TODO: open a native file dialog and load the selected CSV
                // report into application state.
                Task::none()
            }
        }
    }

    /// Build the root view from the current state.
    fn view(&self) -> Element<'_, Message> {
        let status = match &self.loaded_report {
            Some(path) => text(format!("Loaded: {path}")),
            None => text("No report loaded."),
        };

        let content = column![
            text("Match Report Analyzer").size(28),
            status,
            button("Open report…").on_press(Message::OpenReportPressed),
        ]
        .spacing(16);

        container(content).padding(24).into()
    }
}
