use chrono::{Datelike, NaiveDate};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "advent-calendar",
    version,
    about = "View and update a Google Sheets habit calendar"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}
#[derive(Subcommand)]
pub(crate) enum Command {
    /// Save spreadsheet and authentication settings for this user.
    Setup {
        /// Google Sheets ID from the spreadsheet URL.
        #[arg(long)]
        spreadsheet_id: String,
        /// Google OAuth desktop client JSON file.
        #[arg(long)]
        oauth_client_secret: Option<PathBuf>,
        /// Service account JSON key for automation or a shared calendar identity.
        #[arg(long)]
        service_account_key: Option<PathBuf>,
    },
    /// Display a week (defaults to the current week).
    Show {
        /// Any date in the week to display (YYYY-MM-DD).
        #[arg(long)]
        week: Option<NaiveDate>,
    },
    /// Check or uncheck a task for a date.
    Complete {
        /// Date to update (defaults to today).
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long)]
        task: String,
        /// Clear the checkbox instead of checking it.
        #[arg(long)]
        uncheck: bool,
    },
    /// Initialize a month tab using a JSON profile template.
    MonthInit {
        /// Month to create (YYYY-MM; defaults to the current month).
        #[arg(long, value_parser = parse_month)]
        month: Option<(i32, u32)>,
        /// JSON profile (defaults to the user template in the config directory).
        #[arg(long)]
        template: Option<PathBuf>,
    },
}
#[derive(Deserialize)]
pub(crate) struct ProfileTemplate {
    pub(crate) tasks: Vec<TaskTemplate>,
}
pub(crate) fn parse_month(input: &str) -> std::result::Result<(i32, u32), String> {
    let date = NaiveDate::parse_from_str(&format!("{input}-01"), "%Y-%m-%d")
        .map_err(|_| "expected YYYY-MM".to_string())?;
    Ok((date.year(), date.month()))
}

#[derive(Deserialize)]
pub(crate) struct TaskTemplate {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) emoji: String,
    pub(crate) color: String,
    pub(crate) weekly_target: usize,
}
