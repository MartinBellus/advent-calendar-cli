# Advent Calendar CLI

A command-line app for viewing and updating a Google Sheets habit calendar.

## Install

From a checkout of this repository, install the executable with Rust and Cargo:

```sh
cargo install --path . --locked
```

This installs `advent-calendar` into Cargo's binary directory (usually `~/.cargo/bin`). Make sure that directory is on your `PATH`.

## First-Time Setup

Enable the Google Sheets API in a Google Cloud project. For personal use, create an OAuth client with application type **Desktop app** and download its client JSON file. Google sign-in runs the first time you use a Sheets command; it prints a URL to open in your browser and stores refresh tokens in the operating system credential store (macOS Keychain, Windows Credential Manager, or Linux Secret Service).

Save the spreadsheet ID from its URL and the OAuth client file path:

```sh
advent-calendar setup \
  --spreadsheet-id 'your-spreadsheet-id' \
  --oauth-client-secret "$HOME/Downloads/client_secret.json"
```

You can instead use a service account for automation or a shared calendar identity:

```sh
advent-calendar setup \
  --spreadsheet-id 'your-spreadsheet-id' \
  --service-account-key "$HOME/.config/advent-calendar/service-account.json"
```

For service-account mode, share the spreadsheet with the service account's `client_email` and grant Editor access. Keep the JSON key outside the repository; setup stores only its path.

Configuration is saved as `config.json` under the platform user config directory: `$XDG_CONFIG_HOME/advent-calendar` (normally `~/.config/advent-calendar`) on Linux, `~/Library/Application Support/advent-calendar` on macOS, or `%APPDATA%\\advent-calendar` on Windows. The default template is installed at `templates/default.json` in that directory. OAuth refresh tokens are stored in the OS credential store; Linux needs a Secret Service provider such as GNOME Keyring or KWallet. The app also accepts `GOOGLE_SHEETS_ID`, `GOOGLE_OAUTH_CLIENT_SECRET`, and `GOOGLE_SERVICE_ACCOUNT_KEY` as environment overrides. It does not load `.env` files automatically.

## Commands

```sh
advent-calendar show
advent-calendar show --week 2026-10-08
advent-calendar complete --task Physical
advent-calendar complete --date 2026-10-08 --task Physical
advent-calendar complete --date 2026-10-08 --task Physical --uncheck
advent-calendar month-init
advent-calendar month-init --month 2026-11 --template profile.example.json
```

`show` defaults to the current week, Monday through Sunday, and prints a compact table. `complete` defaults to today; task matching is case-insensitive and accepts a label with or without its trailing emoji. `month-init` defaults to the current month and the user's default profile. It refuses to replace an existing tab.

## Profiles and Sheet Layout

Setup installs the example profile at `templates/default.json` in the user config directory if it does not already exist. Edit that copy to customize `month-init`; updates never overwrite it. Each profile has a `tasks` array with unique task names, optional emojis, `#RRGGBB` colors, and weekly targets from 0 to 7. Pass `--template FILE` to use another profile.

New month tabs use weekday columns B:H and task names in column A. They have native checkboxes for in-month days, bold task names, and task colors on labels and active days; unused days stay uncolored. Column I displays a centered `🎉🎉🎉` with a light-green background when a weekly target is met. Weekday columns are 87 pixels wide. The weekday header row and task-name column stay frozen.

Existing tabs should use English `Month-Year` names such as `October-2026` and the same weekly layout for `show` and `complete` to locate cells.
