use crate::{
    calendar::{row_dates, tab_name},
    cli::{ProfileTemplate, TaskTemplate},
    sheets::{Hub, quote_tab, read_values},
};
use anyhow::{Context, Result, bail};
use serde_json::Value;

pub(crate) async fn load_from_month(
    hub: &Hub,
    spreadsheet_id: &str,
    (year, month): (i32, u32),
) -> Result<ProfileTemplate> {
    let title = tab_name(year, month);
    let values = read_values(hub, spreadsheet_id, &title).await?;
    let header = values
        .iter()
        .position(|row| row_dates(row, year, month).iter().any(Option::is_some))
        .with_context(|| format!("No calendar date headers found in tab '{title}'"))?;
    let end = values
        .iter()
        .enumerate()
        .skip(header + 1)
        .find(|(_, row)| row_dates(row, year, month).iter().any(Option::is_some))
        .map(|(index, _)| index)
        .unwrap_or(values.len());

    let colors = read_first_column_colors(hub, spreadsheet_id, &title).await?;
    let mut tasks = Vec::new();
    for (row_index, row) in values.iter().enumerate().take(end).skip(header + 1) {
        let Some(name) = row
            .first()
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        let (name, emoji) = split_trailing_emoji(name);
        if tasks.iter().any(|task: &TaskTemplate| task.name == name) {
            continue;
        }
        tasks.push(TaskTemplate {
            name: name.to_owned(),
            emoji,
            color: colors
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| "#ffffff".to_owned()),
            // Weekly targets are profile metadata and cannot be reliably inferred from a
            // formatted value cell, so imported tasks start with no weekly requirement.
            weekly_target: 0,
        });
    }
    if tasks.is_empty() {
        bail!("No task rows found in the first calendar week of tab '{title}'");
    }
    Ok(ProfileTemplate { tasks })
}

fn split_trailing_emoji(label: &str) -> (&str, String) {
    let chars: Vec<(usize, char)> = label.char_indices().collect();
    let mut index = chars.len();
    let mut found_emoji = false;
    while index > 0 {
        let character = chars[index - 1].1;
        if is_emoji_base(character) || is_emoji_modifier(character) {
            found_emoji = true;
            index -= 1;
        } else if matches!(character as u32, 0xfe0e..=0xfe0f | 0x200d | 0x20e3) {
            index -= 1;
        } else {
            break;
        }
    }
    if !found_emoji {
        return (label.trim(), String::new());
    }
    let start = chars
        .get(index)
        .map(|(offset, _)| *offset)
        .unwrap_or(label.len());
    (label[..start].trim_end(), label[start..].trim().to_owned())
}

fn is_emoji_base(character: char) -> bool {
    matches!(character as u32, 0x1f000..=0x1faff | 0x2600..=0x27bf | 0x2300..=0x23ff)
}

fn is_emoji_modifier(character: char) -> bool {
    matches!(character as u32, 0x1f3fb..=0x1f3ff)
}

async fn read_first_column_colors(
    hub: &Hub,
    spreadsheet_id: &str,
    title: &str,
) -> Result<Vec<String>> {
    let range = format!("{}!A1:A1000", quote_tab(title));
    let (.., spreadsheet) = hub
        .spreadsheets()
        .get(spreadsheet_id)
        .include_grid_data(true)
        .add_ranges(&range)
        .param(
            "fields",
            "sheets(data(rowData(values(effectiveFormat(backgroundColor,backgroundColorStyle),userEnteredFormat(backgroundColor,backgroundColorStyle)))))",
        )
        .doit()
        .await
        .with_context(|| format!("Could not read task colors from tab '{title}'"))?;
    let response = serde_json::to_value(spreadsheet)?;
    let colors = response
        .pointer("/sheets/0/data/0/rowData")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| cell_background_color(&row).unwrap_or_else(|| "#ffffff".to_owned()))
        .collect();
    Ok(colors)
}

fn cell_background_color(row: &Value) -> Option<String> {
    for format in ["effectiveFormat", "userEnteredFormat"] {
        let prefix = format!("/values/0/{format}");
        for field in ["backgroundColorStyle", "backgroundColor"] {
            if let Some(color) = row.pointer(&format!("{prefix}/{field}")) {
                if let Some(hex) = color.get("rgbColor").and_then(rgb_to_hex) {
                    return Some(hex);
                }
            }
        }
    }
    None
}

fn rgb_to_hex(rgb: &Value) -> Option<String> {
    let channel = |name: &str| -> u8 {
        let value = rgb
            .get(name)
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        (value * 255.0).round() as u8
    };
    rgb.as_object()?;
    Some(format!(
        "#{:02x}{:02x}{:02x}",
        channel("red"),
        channel("green"),
        channel("blue")
    ))
}
