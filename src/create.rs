use crate::calendar::tab_name;
use crate::cli::ProfileTemplate;
use crate::sheets::{Hub, quote_tab};
use anyhow::{Context, Result, bail};
use chrono::{Datelike, Duration, NaiveDate};
use google_sheets4::api::{BatchUpdateSpreadsheetRequest, ValueRange};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

const COLUMN_COUNT: usize = 11; // A:K, preserving the existing sheet width.

pub(crate) async fn create_month(
    hub: &Hub,
    spreadsheet_id: &str,
    (year, month): (i32, u32),
    template_path: &PathBuf,
) -> Result<()> {
    let raw_template = fs::read_to_string(template_path)
        .with_context(|| format!("Could not read template {}", template_path.display()))?;
    let template: ProfileTemplate = serde_json::from_str(&raw_template)
        .with_context(|| format!("Invalid template JSON in {}", template_path.display()))?;
    validate_template(&template)?;
    let title = tab_name(year, month);
    let (.., spreadsheet) = hub
        .spreadsheets()
        .get(spreadsheet_id)
        .doit()
        .await
        .context("Could not inspect spreadsheet tabs")?;
    if spreadsheet
        .sheets
        .unwrap_or_default()
        .iter()
        .filter_map(|sheet| sheet.properties.as_ref())
        .any(|properties| properties.title.as_deref() == Some(&title))
    {
        bail!("Tab '{title}' already exists; refusing to overwrite it");
    }

    let create = BatchUpdateSpreadsheetRequest {
        requests: Some(vec![serde_json::from_value(json!({
            "addSheet": {
                "properties": {
                    "title": title,
                    "gridProperties": {
                        "rowCount": 100,
                        "columnCount": COLUMN_COUNT,
                        "frozenRowCount": 1,
                        "frozenColumnCount": 1
                    }
                }
            }
        }))?]),
        ..Default::default()
    };
    hub.spreadsheets()
        .batch_update(create, spreadsheet_id)
        .doit()
        .await
        .with_context(|| format!("Could not create tab '{title}'"))?;

    let sheet_id = get_sheet_id(hub, spreadsheet_id, &title).await?;
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("validated month");
    let offset = first.weekday().num_days_from_monday() as usize;
    let next_month = first
        .with_day(28)
        .expect("valid day")
        .checked_add_signed(Duration::days(4))
        .expect("valid month boundary")
        .with_day(1)
        .expect("first day of next month");
    let day_count = next_month.pred_opt().expect("last day of month").day();
    let week_count = (offset + day_count as usize).div_ceil(7);
    let block_height = template.tasks.len() + 1;
    let total_rows = 1 + week_count * block_height;
    let mut grid = vec![vec![Value::String(String::new()); COLUMN_COUNT]; total_rows];
    for (weekday, name) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .into_iter()
        .enumerate()
    {
        grid[0][weekday + 1] = Value::String(name.to_string());
    }
    for week in 0..week_count {
        let header_row = 1 + week * block_height;
        for weekday in 0..7 {
            let day_number = (week * 7 + weekday + 1) as i32 - offset as i32;
            if (1..=day_count as i32).contains(&day_number) {
                let date = NaiveDate::from_ymd_opt(year, month, day_number as u32).unwrap();
                grid[header_row][weekday + 1] =
                    Value::String(format!("{}.", date.format("%-d.%-m")));
            }
        }
        for (task_idx, task) in template.tasks.iter().enumerate() {
            let row = header_row + task_idx + 1;
            grid[row][0] = Value::String(
                format!("{} {}", task.name.trim(), task.emoji.trim())
                    .trim()
                    .to_string(),
            );
            for weekday in 0..7 {
                let day_number = (week * 7 + weekday + 1) as i32 - offset as i32;
                if (1..=day_count as i32).contains(&day_number) {
                    grid[row][weekday + 1] = Value::Bool(false);
                }
            }
            grid[row][8] = Value::String(format!(
                "=IF(COUNTIF(B{}:H{},TRUE)>={},\"🎉🎉🎉\",\"\")",
                row + 1,
                row + 1,
                task.weekly_target
            ));
        }
    }

    let data_range = format!("{}!A1:K{}", quote_tab(&title), total_rows);
    let data = ValueRange {
        major_dimension: Some("ROWS".into()),
        values: Some(grid),
        ..Default::default()
    };
    hub.spreadsheets()
        .values_update(data, spreadsheet_id, &data_range)
        .value_input_option("USER_ENTERED")
        .doit()
        .await
        .with_context(|| format!("Could not populate tab '{title}'"))?;

    let requests = create_sheet_format_requests(sheet_id, offset, day_count, &template);
    if !requests.is_empty() {
        let format_request = BatchUpdateSpreadsheetRequest {
            requests: Some(requests),
            ..Default::default()
        };
        hub.spreadsheets()
            .batch_update(format_request, spreadsheet_id)
            .doit()
            .await
            .with_context(|| {
                format!("Could not apply checkbox and color formatting to '{title}'")
            })?;
    }
    println!("Created tab '{title}' from {}", template_path.display());
    Ok(())
}

pub(crate) async fn get_sheet_id(hub: &Hub, spreadsheet_id: &str, title: &str) -> Result<i64> {
    let (.., spreadsheet) = hub
        .spreadsheets()
        .get(spreadsheet_id)
        .doit()
        .await
        .context("Could not retrieve newly created tab")?;
    spreadsheet
        .sheets
        .unwrap_or_default()
        .iter()
        .filter_map(|sheet| sheet.properties.as_ref())
        .find(|properties| properties.title.as_deref() == Some(title))
        .and_then(|properties| properties.sheet_id)
        .map(i64::from)
        .context("The new tab was created but its sheet ID was not returned")
}

fn validate_template(template: &ProfileTemplate) -> Result<()> {
    if template.tasks.is_empty() {
        bail!("Template must contain at least one task");
    }
    let mut names = Vec::new();
    for task in &template.tasks {
        if task.name.trim().is_empty() {
            bail!("Task names cannot be empty");
        }
        if names.iter().any(|name: &&str| *name == task.name.trim()) {
            bail!("Task names must be unique in the template: '{}'", task.name);
        }
        if task.weekly_target > 7 {
            bail!("Weekly target for '{}' must be between 0 and 7", task.name);
        }
        parse_color(&task.color)
            .with_context(|| format!("Invalid color for task '{}'", task.name))?;
        names.push(task.name.trim());
    }
    Ok(())
}

fn parse_color(hex: &str) -> Result<(f32, f32, f32)> {
    let value = hex
        .strip_prefix('#')
        .context("color must use #RRGGBB format")?;
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("color must use #RRGGBB format");
    }
    let r = u8::from_str_radix(&value[0..2], 16)? as f32 / 255.0;
    let g = u8::from_str_radix(&value[2..4], 16)? as f32 / 255.0;
    let b = u8::from_str_radix(&value[4..6], 16)? as f32 / 255.0;
    Ok((r, g, b))
}

fn create_sheet_format_requests(
    sheet_id: i64,
    offset: usize,
    days: u32,
    template: &ProfileTemplate,
) -> Vec<google_sheets4::api::Request> {
    let weeks = (offset + days as usize).div_ceil(7);
    let block = template.tasks.len() + 1;
    let mut requests = Vec::new();
    let mut celebration_ranges = Vec::new();
    requests.push(
        serde_json::from_str(
            &json!({
                "updateDimensionProperties": {
                    "range": {
                        "sheetId": sheet_id,
                        "dimension": "COLUMNS",
                        "startIndex": 1,
                        "endIndex": 8
                    },
                    "properties": {"pixelSize": 87},
                    "fields": "pixelSize"
                }
            })
            .to_string(),
        )
        .expect("valid weekday column width request"),
    );
    for week in 0..weeks {
        let header = 1 + week * block;
        for (task_idx, task) in template.tasks.iter().enumerate() {
            let row = header + task_idx + 1;
            let color = parse_color(&task.color).expect("template colors validated");
            requests.push(serde_json::from_str(&json!({
                "repeatCell": {
                    "range": {
                        "sheetId": sheet_id,
                        "startRowIndex": row,
                        "endRowIndex": row + 1,
                        "startColumnIndex": 0,
                        "endColumnIndex": 1
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "backgroundColor": {"red": color.0, "green": color.1, "blue": color.2},
                            "textFormat": {"bold": true}
                        }
                    },
                    "fields": "userEnteredFormat.backgroundColor,userEnteredFormat.textFormat.bold"
                }
            }).to_string()).expect("valid repeatCell request"));

            requests.push(
                serde_json::from_str(
                    &json!({
                        "repeatCell": {
                            "range": {
                                "sheetId": sheet_id,
                                "startRowIndex": row,
                                "endRowIndex": row + 1,
                                "startColumnIndex": 8,
                                "endColumnIndex": 9
                            },
                            "cell": {
                                "userEnteredFormat": {
                                    "horizontalAlignment": "CENTER",
                                    "verticalAlignment": "MIDDLE"
                                }
                            },
                            "fields": "userEnteredFormat.horizontalAlignment,userEnteredFormat.verticalAlignment"
                        }
                    })
                    .to_string(),
                )
                .expect("valid celebration alignment request"),
            );
            celebration_ranges.push(json!({
                "sheetId": sheet_id,
                "startRowIndex": row,
                "endRowIndex": row + 1,
                "startColumnIndex": 8,
                "endColumnIndex": 9
            }));

            for weekday in 0..7 {
                let day_number = (week * 7 + weekday + 1) as i32 - offset as i32;
                if (1..=days as i32).contains(&day_number) {
                    requests.push(
                        serde_json::from_str(
                            &json!({
                                "repeatCell": {
                                    "range": {
                                        "sheetId": sheet_id,
                                        "startRowIndex": row,
                                        "endRowIndex": row + 1,
                                        "startColumnIndex": weekday + 1,
                                        "endColumnIndex": weekday + 2
                                    },
                                    "cell": {
                                        "userEnteredFormat": {
                                            "backgroundColor": {
                                                "red": color.0,
                                                "green": color.1,
                                                "blue": color.2
                                            }
                                        }
                                    },
                                    "fields": "userEnteredFormat.backgroundColor"
                                }
                            })
                            .to_string(),
                        )
                        .expect("valid weekday tint request"),
                    );
                    requests.push(
                        serde_json::from_str(
                            &json!({
                                "setDataValidation": {
                                    "range": {
                                        "sheetId": sheet_id,
                                        "startRowIndex": row,
                                        "endRowIndex": row + 1,
                                        "startColumnIndex": weekday + 1,
                                        "endColumnIndex": weekday + 2
                                    },
                                    "rule": {
                                        "condition": {"type": "BOOLEAN"},
                                        "strict": true,
                                        "showCustomUi": true
                                    }
                                }
                            })
                            .to_string(),
                        )
                        .expect("valid checkbox request"),
                    );
                }
            }
        }
    }
    requests.push(
        serde_json::from_str(
            &json!({
                "addConditionalFormatRule": {
                    "rule": {
                        "ranges": celebration_ranges,
                        "booleanRule": {
                            "condition": {
                                "type": "TEXT_EQ",
                                "values": [{"userEnteredValue": "🎉🎉🎉"}]
                            },
                            "format": {
                                "backgroundColor": {
                                    "red": 0.85,
                                    "green": 0.92,
                                    "blue": 0.83
                                }
                            }
                        }
                    },
                    "index": 0
                }
            })
            .to_string(),
        )
        .expect("valid celebration highlighting request"),
    );
    requests
}
