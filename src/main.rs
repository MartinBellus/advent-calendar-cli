mod calendar;
mod cli;
mod config;
mod create;
mod oauth;
mod sheets;

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
            service_account_key,
        } => {
            let using_service_account = service_account_key.is_some();
            let path = config::setup(spreadsheet_id, service_account_key)?;
            println!("Configuration saved to {}", path.display());
            if using_service_account {
                println!("Service-account authentication is configured.");
            } else {
                println!(
                    "The first Google Sheets command will open Google sign-in in your browser."
                );
            }
            return Ok(());
        }
        Command::ResetSettings { yes } => {
            if !yes && !confirm_settings_reset()? {
                println!("Settings reset cancelled.");
                return Ok(());
            }
            oauth::clear_tokens()?;
            config::clear_user_settings()?;
            println!("Saved settings, templates, and Google OAuth tokens were deleted.");
            return Ok(());
        }
        command => command,
    };

    let config = config::AppConfig::load()?;
    let spreadsheet_id = config.spreadsheet_id()?;
    let sheets = connect(&config).await?;

    match command {
        Command::Setup { .. } | Command::ResetSettings { .. } => {
            unreachable!("settings command returned above")
        }
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

fn confirm_settings_reset() -> Result<bool> {
    use std::io::{self, Write};

    print!("Delete saved settings, templates, and Google OAuth tokens? [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
