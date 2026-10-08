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
cargo run -- complete --date 2026-10-08 --task Physical
cargo run -- complete --date 2026-10-08 --task Physical --uncheck
cargo run -- create-month --month 2026-11 --template profile.example.json
```

`show` defaults to the current week, Monday through Sunday. `complete` accepts the task name with or without its trailing emoji. Month tabs use English `Month-Year` names such as `October-2026`. The CLI refuses to replace an existing tab.

## Profile Template

Pass a JSON file containing a `tasks` array. Each task has a unique `name`, optional `emoji`, hex background `color` (`#RRGGBB`), and a `weekly_target` from 0 to 7. New month tabs get one row per task per week, native checkboxes for in-month dates, and a `🎉` marker in column K when that task's weekly target is met. See [profile.example.json](profile.example.json).

The implementation follows the visible layout from the supplied screenshot: weekday columns B:H, labels in A, and the celebration marker in K. It leaves hidden columns I:J untouched. Existing workbook tabs should use the same month-year naming and weekly row layout for `show` and `complete` to locate cells.
