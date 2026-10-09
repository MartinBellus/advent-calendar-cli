# Advent Calendar CLI

View and update your habit calendar in Google Sheets.

This project was programmed with assistance from an AI agent.

## Install

```sh
cargo install --path . --locked
```

Make sure Cargo's binary directory (usually `~/.cargo/bin`) is on your `PATH`.

## Set up

Run setup, sign in through the browser, and choose the spreadsheet to use:

```sh
advent-calendar setup
```

You only need to do this once. Run `advent-calendar reset-settings` to clear your saved settings and sign in again later.

## Use

```sh
# Show this week's calendar
advent-calendar show

# Show the week containing a date
advent-calendar show --week 2026-10-08

# Choose one or more tasks for today (e.g. enter 1,3)
advent-calendar complete

# Mark a named task complete, or clear it with --uncheck
advent-calendar complete --task Physical
advent-calendar complete --date 2026-10-08 --task Physical --uncheck

# Create this month's tab, or choose a month
advent-calendar month-init
advent-calendar month-init --month 2026-11

# Load task names and colors from this month's connected sheet into the default template
advent-calendar load
```

`show` and `complete` default to today. `month-init` defaults to the current month and will not replace an existing tab. Use `--template FILE` with `month-init` to choose a profile.

`load` reads the first calendar week in the current month's tab and overwrites the saved default template with its task names and colors. Use `--month YYYY-MM` to load another month's tab. Imported tasks use a weekly target of 0 because the sheet's celebration formulas do not reliably reveal the intended target.

## Template

`month-init` uses a default template with task names, optional emojis, colors, and weekly targets for the new month. Setup creates a copy at `templates/default.json` in the app's configuration directory. Edit that file to customize future month tabs, or make another JSON template and pass it with `--template FILE`.

For more options, run `advent-calendar --help` or `advent-calendar <command> --help`. Add `-v` before a command to see detailed error information.
