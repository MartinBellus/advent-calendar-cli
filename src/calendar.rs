use crate::sheets::{Hub, quote_tab, read_values};
use anyhow::{Context, Result, bail};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use google_sheets4::api::ValueRange;
use serde_json::Value;

pub(crate) fn tab_name(year: i32, month: u32) -> String {
    NaiveDate::from_ymd_opt(year, month, 1)
        .expect("validated year and month")
        .format("%B-%Y")
        .to_string()
}

fn month_from_title(title: &str) -> Result<(i32, u32)> {
    let date = NaiveDate::parse_from_str(&format!("{}-01", title), "%B-%Y-%d")
        .with_context(|| format!("Tab '{title}' does not use the expected Month-Year naming"))?;
    Ok((date.year(), date.month()))
}

fn parse_cell_date(text: &str, title_year: i32, title_month: u32) -> Option<NaiveDate> {
    for format in ["%Y-%m-%d", "%d.%m.%Y", "%d.%m.", "%d.%m"] {
        if let Ok(date) = NaiveDate::parse_from_str(text.trim(), format) {
            if format == "%d.%m." || format == "%d.%m" {
                let month = date.month();
                let year = adjacent_year(title_year, title_month, month);
                return NaiveDate::from_ymd_opt(year, month, date.day());
            }
            return Some(date);
        }
    }
    let mut parts = text.trim().trim_end_matches('.').split('.');
    let day = parts.next()?.parse::<u32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let year = parts
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| adjacent_year(title_year, title_month, month));
    NaiveDate::from_ymd_opt(year, month, day)
}

fn adjacent_year(year: i32, month: u32, cell_month: u32) -> i32 {
    if month == 1 && cell_month == 12 {
        year - 1
    } else if month == 12 && cell_month == 1 {
        year + 1
    } else {
        year
    }
}

pub(crate) fn row_dates(row: &[String], year: i32, month: u32) -> Vec<Option<NaiveDate>> {
    (1..=7)
        .map(|column| {
            row.get(column)
                .and_then(|value| parse_cell_date(value, year, month))
        })
        .collect()
}

pub(crate) async fn show_week(hub: &Hub, spreadsheet_id: &str, selected: NaiveDate) -> Result<()> {
    let title = tab_name(selected.year(), selected.month());
    let values = read_values(hub, spreadsheet_id, &title).await?;
    let (year, month) = month_from_title(&title)?;
    let monday = selected - Duration::days(selected.weekday().num_days_from_monday() as i64);
    let target_dates: Vec<_> = (0..7).map(|day| monday + Duration::days(day)).collect();

    let Some((header_row, dates)) = values.iter().enumerate().find_map(|(idx, row)| {
        let dates = row_dates(row, year, month);
        dates
            .iter()
            .flatten()
            .any(|date| target_dates.contains(date))
            .then_some((idx, dates))
    }) else {
        bail!(
            "No calendar week containing {} was found in tab '{title}'",
            selected
        );
    };

    let week_rows: Vec<_> = values
        .iter()
        .skip(header_row + 1)
        .take_while(|row| !row_dates(row, year, month).iter().any(Option::is_some))
        .filter_map(|row| {
            row.first()
                .filter(|label| !label.trim().is_empty())
                .map(|label| (row, label))
        })
        .collect();
    if week_rows.is_empty() {
        bail!("No task rows found for the week of {monday} in tab '{title}'");
    }

    let task_width = week_rows
        .iter()
        .map(|(_, label)| terminal_width(label))
        .max()
        .unwrap_or(4)
        .max(terminal_width("Task"));
    println!("{title} · Week of {}", monday.format("%A, %B %-d, %Y"));
    print_task_cell("Task", task_width);
    for date in &target_dates {
        print!(
            " {:^7}|",
            format!("{} {}", weekday_label(date.weekday()), date.format("%d"))
        );
    }
    println!();
    print!("{}-+", "-".repeat(task_width));
    for _ in &target_dates {
        print!("--------+");
    }
    println!();

    for (row, label) in week_rows {
        print_task_cell(label.trim(), task_width);
        for target in &target_dates {
            let checked = dates
                .iter()
                .position(|date| *date == Some(*target))
                .and_then(|day_col| row.get(day_col + 1))
                .is_some_and(|value| value.eq_ignore_ascii_case("TRUE") || value == "✓");
            print!(" {:^7}|", if checked { "✓" } else { "·" });
        }
        println!();
    }
    Ok(())
}

fn print_task_cell(label: &str, width: usize) {
    print!(
        "{label}{} |",
        " ".repeat(width.saturating_sub(terminal_width(label)))
    );
}

fn terminal_width(text: &str) -> usize {
    text.chars().map(terminal_char_width).sum()
}

fn terminal_char_width(character: char) -> usize {
    let code = character as u32;
    if character.is_control()
        || matches!(
            code,
            0x0300..=0x036f | 0x200d | 0xfe00..=0xfe0f | 0x1f3fb..=0x1f3ff
        )
    {
        return 0;
    }
    if matches!(
        code,
        0x1100..=0x115f
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7a3
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe19
            | 0xfe30..=0xfe6f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
            | 0x1f300..=0x1faff
            | 0x20000..=0x3fffd
    ) {
        2
    } else {
        1
    }
}

fn weekday_label(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

pub(crate) async fn set_completion(
    hub: &Hub,
    spreadsheet_id: &str,
    date: NaiveDate,
    task: &str,
    checked: bool,
) -> Result<()> {
    let title = tab_name(date.year(), date.month());
    let values = read_values(hub, spreadsheet_id, &title).await?;
    let (year, month) = month_from_title(&title)?;
    let header = values
        .iter()
        .enumerate()
        .find_map(|(idx, row)| {
            row_dates(row, year, month)
                .iter()
                .position(|cell_date| *cell_date == Some(date))
                .map(|column| (idx, column))
        })
        .with_context(|| format!("Date {date} was not found in tab '{title}'"))?;
    let next_header = values
        .iter()
        .enumerate()
        .skip(header.0 + 1)
        .find(|(_, row)| row_dates(row, year, month).iter().any(Option::is_some))
        .map(|(idx, _)| idx)
        .unwrap_or(values.len());
    let matches: Vec<(usize, usize)> = values[header.0 + 1..next_header]
        .iter()
        .enumerate()
        .filter_map(|(offset, row)| {
            row.first()
                .is_some_and(|value| labels_match(value, task))
                .then_some((header.0 + offset + 2, header.1 + 2))
        })
        .collect();
    if matches.is_empty() {
        bail!("Task '{task}' or date {date} was not found in tab '{title}'");
    }
    if matches.len() > 1 {
        bail!("Task '{task}' occurs more than once for {date}; the sheet layout is ambiguous");
    }
    let (row, column) = matches[0];
    let range = format!("{}!{}{}", quote_tab(&title), column_label(column), row);
    let request = ValueRange {
        major_dimension: Some("ROWS".to_string()),
        values: Some(vec![vec![Value::Bool(checked)]]),
        ..Default::default()
    };
    hub.spreadsheets()
        .values_update(request, spreadsheet_id, &range)
        .value_input_option("USER_ENTERED")
        .doit()
        .await
        .with_context(|| format!("Could not update {task} for {date}"))?;
    println!(
        "{} '{task}' for {date}",
        if checked { "Checked" } else { "Unchecked" }
    );
    Ok(())
}

fn column_label(mut column: usize) -> String {
    let mut label = String::new();
    while column > 0 {
        let remainder = (column - 1) % 26;
        label.insert(0, (b'A' + remainder as u8) as char);
        column = (column - 1) / 26;
    }
    label
}

fn labels_match(label: &str, requested: &str) -> bool {
    let label = label.trim().to_lowercase();
    let requested = requested.trim().to_lowercase();
    label == requested
        || label.strip_prefix(&requested).is_some_and(|suffix| {
            suffix.chars().next().is_some_and(char::is_whitespace)
                && suffix
                    .trim()
                    .chars()
                    .all(|character| !character.is_alphanumeric())
        })
}
