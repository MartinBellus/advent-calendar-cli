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
    /// Display a week (defaults to the current week).
    Show {
        /// Any date in the week to display (YYYY-MM-DD).
        #[arg(long)]
        week: Option<NaiveDate>,
    },
    /// Check or uncheck a task for a date.
    Complete {
        #[arg(long)]
        date: NaiveDate,
        #[arg(long)]
        task: String,
        /// Clear the checkbox instead of checking it.
        #[arg(long)]
        uncheck: bool,
    },
    /// Create a month tab using a JSON profile template.
    CreateMonth {
        /// Month to create (YYYY-MM).
        #[arg(long, value_parser = parse_month)]
        month: (i32, u32),
        /// JSON file containing task names, emojis, colors, and weekly targets.
        #[arg(long)]
        template: PathBuf,
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
