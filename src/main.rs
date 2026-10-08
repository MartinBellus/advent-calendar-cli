mod calendar;
mod cli;
mod create;
mod sheets;

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use cli::{Cli, Command};
use sheets::connect;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let sheets = connect().await?;
    let spreadsheet_id = env::var("GOOGLE_SHEETS_ID")
        .context("Set GOOGLE_SHEETS_ID to the ID of your spreadsheet")?;

    match cli.command {
        Command::Show { week } => {
            calendar::show_week(
                &sheets,
                &spreadsheet_id,
                week.unwrap_or_else(|| Local::now().date_naive()),
            )
            .await
        }
        Command::Complete {
            date,
            task,
            uncheck,
        } => calendar::set_completion(&sheets, &spreadsheet_id, date, &task, !uncheck).await,
        Command::CreateMonth { month, template } => {
            create::create_month(&sheets, &spreadsheet_id, month, &template).await
        }
    }
}
