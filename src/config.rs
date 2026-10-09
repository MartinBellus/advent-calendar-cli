use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct AppConfig {
    pub(crate) spreadsheet_id: Option<String>,
    pub(crate) service_account_key: Option<PathBuf>,
}

impl AppConfig {
    pub(crate) fn load() -> Result<Self> {
        let path = config_path()?;
        let config = match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents)
                .with_context(|| format!("Could not parse configuration at {}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                return Err(error).with_context(|| format!("Could not read {}", path.display()));
            }
        };
        Ok(config)
    }

    pub(crate) fn spreadsheet_id(&self) -> Result<String> {
        env::var("GOOGLE_SHEETS_ID")
            .ok()
            .or_else(|| self.spreadsheet_id.clone())
            .filter(|id| !id.trim().is_empty())
            .context("Run `advent-calendar setup` to sign in and choose a spreadsheet")
    }

    pub(crate) fn service_account_key(&self) -> Option<PathBuf> {
        env::var_os("GOOGLE_SERVICE_ACCOUNT_KEY")
            .map(PathBuf::from)
            .or_else(|| self.service_account_key.clone())
    }

    pub(crate) fn default_template_path(&self) -> Result<PathBuf> {
        let path = config_dir()?.join("templates").join("default.json");
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Could not create template directory {}", parent.display())
                })?;
            }
            fs::write(&path, include_str!("../profile.example.json")).with_context(|| {
                format!(
                    "Could not install the default template at {}",
                    path.display()
                )
            })?;
        }
        Ok(path)
    }
}

pub(crate) fn setup(
    spreadsheet_id: String,
    service_account_key: Option<PathBuf>,
) -> Result<PathBuf> {
    if spreadsheet_id.trim().is_empty() {
        bail!("Spreadsheet ID cannot be empty");
    }
    let service_account_key = canonicalize_option(service_account_key, "service account key")?;
    let config = AppConfig {
        spreadsheet_id: Some(spreadsheet_id.trim().to_owned()),
        service_account_key,
    };
    let path = write_config(&config)?;
    config.default_template_path()?;
    Ok(path)
}

fn write_config(config: &AppConfig) -> Result<PathBuf> {
    let path = config_path()?;
    let directory = path.parent().context("Configuration path has no parent")?;
    fs::create_dir_all(directory).with_context(|| {
        format!(
            "Could not create configuration directory {}",
            directory.display()
        )
    })?;
    let contents = serde_json::to_string_pretty(config)?;
    fs::write(&path, contents).with_context(|| format!("Could not write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn clear_user_settings() -> Result<()> {
    let directory = config_dir()?;
    match fs::remove_dir_all(&directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "Could not remove settings directory {}",
                directory.display()
            )
        }),
    }
}

fn canonicalize_option(path: Option<PathBuf>, description: &str) -> Result<Option<PathBuf>> {
    path.map(|path| {
        fs::canonicalize(&path)
            .with_context(|| format!("Could not find {description} at {}", path.display()))
    })
    .transpose()
}

fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.json"))
}

fn config_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = env::var_os("APPDATA") {
            return Ok(PathBuf::from(app_data).join("advent-calendar"));
        }
        return Ok(home_dir()?
            .join("AppData")
            .join("Roaming")
            .join("advent-calendar"));
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(home_dir()?
            .join("Library")
            .join("Application Support")
            .join("advent-calendar"));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
            let config_home = PathBuf::from(config_home);
            if config_home.is_absolute() {
                return Ok(config_home.join("advent-calendar"));
            }
        }
        Ok(home_dir()?.join(".config").join("advent-calendar"))
    }
}

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("Could not determine the user's home directory")
}
