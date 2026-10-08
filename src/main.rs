mod calendar;
mod cli;
mod config;
mod create;
mod sheets;
mod token_store;

use anyhow::Result;
use chrono::{Datelike, Local};
use clap::Parser;
use cli::{Cli, Command};
use sheets::connect;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let command = match cli.command {
        Command::Setup {
            spreadsheet_id,
            oauth_client_secret,
            service_account_key,
        } => {
            let path = config::setup(spreadsheet_id, oauth_client_secret, service_account_key)?;
            println!("Configuration saved to {}", path.display());
            println!("The first Google Sheets command will sign in if OAuth is configured.");
            return Ok(());
        }
        command => command,
    };

    let config = config::AppConfig::load()?;
    let spreadsheet_id = config.spreadsheet_id()?;
    let sheets = connect(&config).await?;

    match command {
        Command::Setup { .. } => unreachable!("setup command returned above"),
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
        } => {
            let date = date.unwrap_or_else(|| Local::now().date_naive());
            calendar::set_completion(&sheets, &spreadsheet_id, date, &task, !uncheck).await
        }
        Command::MonthInit { month, template } => {
            let month = month.unwrap_or_else(|| {
                let today = Local::now().date_naive();
                (today.year(), today.month())
            });
            let template = match template {
                Some(template) => template,
                None => config.default_template_path()?,
            };
            create::create_month(&sheets, &spreadsheet_id, month, &template).await
        }
    }
}
