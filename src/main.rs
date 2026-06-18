// Don't spawn a console window alongside the GUI on Windows release builds.
// Kept in debug builds so `println!`/panics remain visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! match-report-analyzer — a cross-platform native desktop GUI for analyzing
//! Physna geometric match-report CSV exports.
//!
//! Built with [Iced](https://iced.rs/). A loaded CSV is held in an in-memory
//! SQLite table (see `match_report_analyzer::store`); both the structured
//! filter builder and the raw SQL box (see `match_report_analyzer::query`)
//! compile to SQL that runs against it, feeding one read-only result grid.

use std::path::PathBuf;

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Column, Row, button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Element, Length, Task, Theme};

use match_report_analyzer::query::{Combinator, Condition, FilterBuilder, Operator};
use match_report_analyzer::store::{DataStore, QueryResult, TABLE, quote_ident};

/// Maximum number of result rows rendered at once. Iced does not virtualize
/// large tables, so we cap the displayed rows for responsiveness; the status
/// bar reports how many matched in total.
const ROW_DISPLAY_CAP: usize = 500;

const COLUMN_WIDTH: f32 = 180.0;
const CELL_CHAR_CAP: usize = 48;

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .theme(|_| Theme::Dark)
        .run()
}

/// Which query-authoring tab is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Builder,
    RawSql,
}

/// Sort direction applied via a header click (builder mode only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    fn sql(self) -> &'static str {
        match self {
            SortDir::Asc => "ASC",
            SortDir::Desc => "DESC",
        }
    }

    fn indicator(self) -> &'static str {
        match self {
            SortDir::Asc => " ▲",
            SortDir::Desc => " ▼",
        }
    }
}

/// Status line content shown beneath the toolbar.
#[derive(Debug, Clone, Default)]
enum Status {
    #[default]
    Idle,
    Info(String),
    Error(String),
}

/// Top-level application state.
#[derive(Default)]
struct App {
    store: Option<DataStore>,
    file_name: Option<String>,
    total_rows: usize,
    tab: Tab,
    filter: FilterBuilder,
    raw_sql: String,
    sort: Option<(String, SortDir)>,
    result: QueryResult,
    status: Status,
}

#[derive(Debug, Clone)]
enum Message {
    OpenFilePressed,
    FileSelected(Option<PathBuf>),
    TabSelected(Tab),
    AddCondition,
    RemoveCondition(usize),
    ConditionColumnChanged(usize, String),
    ConditionOperatorChanged(usize, Operator),
    ConditionValueChanged(usize, String),
    CombinatorChanged(Combinator),
    ApplyFilter,
    RawSqlChanged(String),
    RunRawSql,
    SortBy(String),
}

impl App {
    fn title(&self) -> String {
        match &self.file_name {
            Some(name) => format!("Match Report Analyzer — {name}"),
            None => "Match Report Analyzer".to_string(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenFilePressed => {
                return Task::perform(pick_file(), Message::FileSelected);
            }
            Message::FileSelected(Some(path)) => self.load_file(path),
            Message::FileSelected(None) => {}
            Message::TabSelected(tab) => self.tab = tab,
            Message::AddCondition => self.filter.conditions.push(Condition::default()),
            Message::RemoveCondition(i) => {
                if i < self.filter.conditions.len() {
                    self.filter.conditions.remove(i);
                }
            }
            Message::ConditionColumnChanged(i, col) => {
                if let Some(c) = self.filter.conditions.get_mut(i) {
                    c.column = Some(col);
                }
            }
            Message::ConditionOperatorChanged(i, op) => {
                if let Some(c) = self.filter.conditions.get_mut(i) {
                    c.operator = Some(op);
                }
            }
            Message::ConditionValueChanged(i, value) => {
                if let Some(c) = self.filter.conditions.get_mut(i) {
                    c.value = value;
                }
            }
            Message::CombinatorChanged(combinator) => self.filter.combinator = combinator,
            Message::ApplyFilter => self.run_query(),
            Message::RawSqlChanged(sql) => self.raw_sql = sql,
            Message::RunRawSql => self.run_query(),
            Message::SortBy(col) => {
                self.sort = match self.sort.take() {
                    Some((c, SortDir::Asc)) if c == col => Some((col, SortDir::Desc)),
                    Some((c, SortDir::Desc)) if c == col => None,
                    _ => Some((col, SortDir::Asc)),
                };
                self.run_query();
            }
        }
        Task::none()
    }

    /// Load a CSV file into a fresh store and run the default query.
    fn load_file(&mut self, path: PathBuf) {
        match DataStore::load_csv(&path) {
            Ok(store) => {
                self.total_rows = store.row_count().unwrap_or(0);
                self.file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                self.raw_sql = format!("SELECT * FROM {TABLE}");
                self.filter = FilterBuilder::default();
                self.sort = None;
                self.store = Some(store);
                self.run_query();
            }
            Err(e) => self.status = Status::Error(format!("Failed to load file: {e}")),
        }
    }

    /// Assemble SQL for the active tab, run it, and capture the result.
    fn run_query(&mut self) {
        let Some(store) = self.store.as_ref() else {
            return;
        };

        let sql = match self.tab {
            Tab::Builder => {
                let mut sql = format!("SELECT * FROM {TABLE}");
                if let Some(clause) = self.filter.where_clause(store.columns()) {
                    sql.push_str(&format!(" WHERE {clause}"));
                }
                if let Some((col, dir)) = &self.sort {
                    sql.push_str(&format!(" ORDER BY {} {}", quote_ident(col), dir.sql()));
                }
                sql
            }
            Tab::RawSql => self.raw_sql.clone(),
        };

        match store.query(&sql) {
            Ok(result) => {
                let matched = result.rows.len();
                self.result = result;
                self.status = Status::Info(format!("{matched} of {} rows", self.total_rows));
            }
            Err(e) => self.status = Status::Error(e.to_string()),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let toolbar = self.view_toolbar();

        let body: Element<'_, Message> = if self.store.is_some() {
            column![self.view_query_panel(), self.view_grid()]
                .spacing(12)
                .height(Length::Fill)
                .into()
        } else {
            container(text("Open a Physna match-report CSV to begin.").size(18))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        container(column![toolbar, self.view_status(), body].spacing(10))
            .padding(16)
            .into()
    }

    fn view_toolbar(&self) -> Element<'_, Message> {
        let file_label = match &self.file_name {
            Some(name) => text(name.clone()),
            None => text("No report loaded").size(14),
        };
        row![
            button("Open report…").on_press(Message::OpenFilePressed),
            file_label,
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn view_status(&self) -> Element<'_, Message> {
        match &self.status {
            Status::Idle => text("").into(),
            Status::Info(msg) => text(msg.clone()).size(14).into(),
            Status::Error(msg) => text(format!("⚠ {msg}")).size(14).style(text::danger).into(),
        }
    }

    fn view_query_panel(&self) -> Element<'_, Message> {
        let tabs = row![
            tab_button("Filter builder", Tab::Builder, self.tab),
            tab_button("SQL", Tab::RawSql, self.tab),
        ]
        .spacing(6);

        let panel: Element<'_, Message> = match self.tab {
            Tab::Builder => self.view_builder(),
            Tab::RawSql => self.view_raw_sql(),
        };

        column![tabs, panel].spacing(10).into()
    }

    fn view_builder(&self) -> Element<'_, Message> {
        let column_names: Vec<String> = self
            .store
            .as_ref()
            .map(|s| s.columns().iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default();

        let mut rows: Vec<Element<'_, Message>> = Vec::new();

        for (i, condition) in self.filter.conditions.iter().enumerate() {
            let column_pick =
                pick_list(column_names.clone(), condition.column.clone(), move |col| {
                    Message::ConditionColumnChanged(i, col)
                })
                .placeholder("column")
                .width(Length::Fixed(220.0));

            let operator_pick = pick_list(&Operator::ALL[..], condition.operator, move |op| {
                Message::ConditionOperatorChanged(i, op)
            })
            .placeholder("operator")
            .width(Length::Fixed(140.0));

            let value: Element<'_, Message> = if condition
                .operator
                .map(Operator::needs_value)
                .unwrap_or(true)
            {
                text_input("value", &condition.value)
                    .on_input(move |v| Message::ConditionValueChanged(i, v))
                    .width(Length::Fixed(200.0))
                    .into()
            } else {
                container(text("")).width(Length::Fixed(200.0)).into()
            };

            rows.push(
                row![
                    column_pick,
                    operator_pick,
                    value,
                    button("✕").on_press(Message::RemoveCondition(i)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .into(),
            );
        }

        let mut controls = Row::new().spacing(8).align_y(iced::Alignment::Center);
        controls = controls.push(button("+ Add condition").on_press(Message::AddCondition));
        if self.filter.conditions.len() > 1 {
            controls = controls.push(text("combine with"));
            controls = controls.push(
                pick_list(
                    &Combinator::ALL[..],
                    Some(self.filter.combinator),
                    Message::CombinatorChanged,
                )
                .width(Length::Fixed(90.0)),
            );
        }
        controls = controls.push(button("Apply filter").on_press(Message::ApplyFilter));

        let mut content = Column::with_children(rows).spacing(8);
        content = content.push(controls);
        content.into()
    }

    fn view_raw_sql(&self) -> Element<'_, Message> {
        column![
            text_input("SELECT * FROM report WHERE …", &self.raw_sql)
                .on_input(Message::RawSqlChanged)
                .on_submit(Message::RunRawSql)
                .width(Length::Fill),
            row![
                button("Run query").on_press(Message::RunRawSql),
                text(format!("Table name: {TABLE}")).size(13),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8)
        .into()
    }

    fn view_grid(&self) -> Element<'_, Message> {
        if self.result.columns.is_empty() {
            return container(text("No results.")).padding(8).into();
        }

        let sortable = matches!(self.tab, Tab::Builder);
        let mut header_cells: Vec<Element<'_, Message>> = Vec::new();
        for name in &self.result.columns {
            let indicator = match &self.sort {
                Some((c, dir)) if c == name => dir.indicator(),
                _ => "",
            };
            let label = format!("{name}{indicator}");
            let cell: Element<'_, Message> = if sortable {
                button(text(label).size(13))
                    .on_press(Message::SortBy(name.clone()))
                    .style(button::text)
                    .width(Length::Fixed(COLUMN_WIDTH))
                    .into()
            } else {
                container(text(label).size(13))
                    .width(Length::Fixed(COLUMN_WIDTH))
                    .padding([4, 8])
                    .into()
            };
            header_cells.push(cell);
        }
        let header = Row::with_children(header_cells);

        let mut body_rows: Vec<Element<'_, Message>> = Vec::new();
        for record in self.result.rows.iter().take(ROW_DISPLAY_CAP) {
            let cells: Vec<Element<'_, Message>> = record
                .iter()
                .map(|value| {
                    container(text(truncate(value)).size(12))
                        .width(Length::Fixed(COLUMN_WIDTH))
                        .padding([3, 8])
                        .into()
                })
                .collect();
            body_rows.push(Row::with_children(cells).into());
        }

        let table = Column::with_children(
            std::iter::once(Element::from(header))
                .chain(body_rows)
                .collect::<Vec<_>>(),
        )
        .spacing(2);

        scrollable(table)
            .direction(Direction::Both {
                vertical: Scrollbar::new(),
                horizontal: Scrollbar::new(),
            })
            .height(Length::Fill)
            .into()
    }
}

/// Open a native file picker for a CSV file, returning the chosen path.
async fn pick_file() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_title("Open match report")
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

/// A tab selector button, highlighted when active.
fn tab_button(label: &str, tab: Tab, active: Tab) -> Element<'_, Message> {
    let style = if tab == active {
        button::primary
    } else {
        button::secondary
    };
    button(text(label))
        .on_press(Message::TabSelected(tab))
        .style(style)
        .into()
}

/// Truncate a cell value for display so wide content doesn't wrap the grid.
fn truncate(value: &str) -> String {
    if value.chars().count() > CELL_CHAR_CAP {
        let mut s: String = value.chars().take(CELL_CHAR_CAP - 1).collect();
        s.push('…');
        s
    } else {
        value.to_string()
    }
}
