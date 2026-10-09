# Advent Calendar CLI

A command-line app for viewing and updating a Google Sheets habit calendar.

## Install

From the repository root, `cargo install --path . --locked` builds this package from the local checkout using `Cargo.lock`, then copies the `advent-calendar` executable to Cargo's binary directory (usually `~/.cargo/bin`). Add that directory to your `PATH`. The OAuth client configuration is embedded at compile time, so an installed binary carries it without needing the repository files at runtime.

```sh
cargo install --path . --locked
```

This installs `advent-calendar` into Cargo's binary directory (usually `~/.cargo/bin`). Make sure that directory is on your `PATH`.

## First-Time Setup

OAuth is the default authentication method; the app includes its public Google OAuth Desktop client ID, so users do not need to create a Google Cloud client. On the first Sheets command the CLI opens Google sign-in in your browser, receives the authorization callback on localhost using PKCE, and stores access and refresh tokens in the operating system credential store (macOS Keychain, Windows Credential Manager, or Linux Secret Service). Expired access tokens refresh automatically. The scope requested is `https://www.googleapis.com/auth/drive.file`.

Run setup with the spreadsheet ID. The selected spreadsheet ID is saved in the user configuration directory. Service-account mode remains available for automation:

```sh
advent-calendar setup --spreadsheet-id 'your-spreadsheet-id'
```

You can instead use a service account for automation or a shared calendar identity:

```sh
advent-calendar setup \
  --spreadsheet-id 'your-spreadsheet-id' \
  --service-account-key "$HOME/.config/advent-calendar/service-account.json"
```

For service-account mode, share the spreadsheet with the service account's `client_email` and grant Editor access. Keep the JSON key outside the repository; setup stores only its path.

Configuration is saved as `config.json` under the platform user config directory: `$XDG_CONFIG_HOME/advent-calendar` (normally `~/.config/advent-calendar`) on Linux, `~/Library/Application Support/advent-calendar` on macOS, or `%APPDATA%\\advent-calendar` on Windows. The default template is installed at `templates/default.json` in that directory. OAuth tokens are stored in the OS credential store; Linux needs a Secret Service provider such as GNOME Keyring or KWallet. The app accepts `GOOGLE_SHEETS_ID` and `GOOGLE_SERVICE_ACCOUNT_KEY` as environment overrides. It does not load `.env` files automatically.

The bundled OAuth client configuration can be overridden for development with a Google Desktop client JSON named `credentials.json` or `client_secret.json` in the current directory. Local client files and all per-user OAuth tokens are ignored by Git. The bundled client configuration is shared app configuration; access and refresh tokens remain private to each user in the OS credential store.

## Commands

```sh
advent-calendar show
advent-calendar show --week 2026-10-08
advent-calendar complete --task Physical
advent-calendar complete --date 2026-10-08 --task Physical
advent-calendar complete --date 2026-10-08 --task Physical --uncheck
advent-calendar month-init
advent-calendar month-init --month 2026-11 --template profile.example.json
advent-calendar reset-settings
```

`reset-settings` asks for confirmation, then deletes the app's saved configuration and templates and removes its OAuth tokens from the OS credential store. Use `reset-settings --yes` to skip the prompt. It does not delete environment overrides or local OAuth client JSON files.

`show` defaults to the current week, Monday through Sunday, and prints a compact table. `complete` defaults to today; task matching is case-insensitive and accepts a label with or without its trailing emoji. `month-init` defaults to the current month and the user's default profile. It refuses to replace an existing tab.

Errors show a short summary by default. Add `-v` (for example, `advent-calendar -v show`) to print the full error chain. If the selected month tab is missing, create it with `advent-calendar month-init`.

## Profiles and Sheet Layout

Setup installs the example profile at `templates/default.json` in the user config directory if it does not already exist. Edit that copy to customize `month-init`; updates never overwrite it. Each profile has a `tasks` array with unique task names, optional emojis, `#RRGGBB` colors, and weekly targets from 0 to 7. Pass `--template FILE` to use another profile.

New month tabs use weekday columns B:H and task names in column A. They have native checkboxes for in-month days, bold task names, and task colors on labels and active days; unused days stay uncolored. Column I displays a centered `🎉🎉🎉` with a light-green background when a weekly target is met. Weekday columns are 87 pixels wide. The weekday header row and task-name column stay frozen.

Existing tabs should use English `Month-Year` names such as `October-2026` and the same weekly layout for `show` and `complete` to locate cells.
