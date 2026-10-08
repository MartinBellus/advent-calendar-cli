# Advent Calendar CLI

A Rust CLI MVP for viewing and updating a month-tab Google Sheets habit calendar.

## Setup

1. Enable the Google Sheets API in a Google Cloud project and create a service account key in JSON format.
2. Share the spreadsheet with the service account email address, granting Editor access (the CLI writes checkboxes and creates tabs).
3. Set the spreadsheet ID and local key path:

   ```sh
   export GOOGLE_SHEETS_ID='your-spreadsheet-id'
   export GOOGLE_SERVICE_ACCOUNT_KEY="$HOME/.config/advent-calendar/service-account.json"
   ```

The key is read locally and must not be committed. The account needs access to the existing spreadsheet.

## Commands

```sh
cargo run -- show
cargo run -- show --week 2026-10-08
cargo run -- complete --task Physical
cargo run -- complete --date 2026-10-08 --task Physical
cargo run -- complete --date 2026-10-08 --task Physical --uncheck
cargo run -- month-init --template profile.example.json
cargo run -- month-init --month 2026-11 --template profile.example.json
```

`show` defaults to the current week, Monday through Sunday, and prints a compact table. `complete` defaults to today; pass `--date` to choose another day. Task matching is case-insensitive and accepts the task name with or without its trailing emoji. `month-init` defaults to the current month; pass `--month YYYY-MM` to choose another. Month tabs use English `Month-Year` names such as `October-2026`. The CLI refuses to replace an existing tab.

## Profile Template

Pass a JSON file containing a `tasks` array. Each task has a unique `name`, optional `emoji`, hex background `color` (`#RRGGBB`), and a `weekly_target` from 0 to 7. New month tabs get one row per task per week, native checkboxes for in-month dates, bold task names on stronger label colors, lighter tints on in-month weekday cells, and a centered `🎉🎉🎉` marker in column I with a light-green background when that task's weekly target is met. Unused days stay uncolored. Weekday columns are narrowed to keep the calendar compact. The weekday header row and task-name column stay frozen while scrolling. See [profile.example.json](profile.example.json).

The implementation uses weekday columns B:H, labels in A, and the celebration marker in I. Existing workbook tabs should use the same month-year naming and weekly row layout for `show` and `complete` to locate cells.
