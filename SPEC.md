# Advent Calendar CLI Specification

## Purpose

Provide an installable, per-user command-line app for viewing and updating a Google Sheets habit calendar. It can initialize month tabs from editable JSON task profiles.

## Installation

- The installed executable is named `advent-calendar`.
- Developers can install the current checkout with `cargo install --path . --locked`.
- Cargo places installed executables in its binary directory, normally `~/.cargo/bin` when installed through rustup. That directory must be on `PATH`.
- Prebuilt release packages may be added later; they are not required for local Cargo installation.

## First-Time Setup and Configuration

Run `advent-calendar setup --spreadsheet-id ID`; OAuth is the default and the app uses its bundled Desktop client configuration. Local `credentials.json` or `client_secret.json` files may override it for development. Service-account mode remains an explicit option and requires `--spreadsheet-id ID --service-account-key FILE`.

- OAuth is the preferred mode for a person using the app on their own computer. On first Sheets use, the CLI starts an authorization-code flow with PKCE, opens the browser, and listens on localhost for the redirect. It requests `https://www.googleapis.com/auth/drive.file`; access and refresh tokens are stored in the OS credential store and expired access tokens are refreshed automatically.
- Service-account credentials remain available for automation or shared-calendar identities. The spreadsheet must be shared with the service account, with edit access for write operations.
- Setup stores the spreadsheet ID and optional service-account key path, not credential contents.
- `GOOGLE_SHEETS_ID` and `GOOGLE_SERVICE_ACCOUNT_KEY` environment variables override corresponding saved settings. The app does not implicitly load `.env` files.
- Configuration and user templates live in the platform user configuration directory. On Linux this is `$XDG_CONFIG_HOME/advent-calendar` or `~/.config/advent-calendar` when `XDG_CONFIG_HOME` is unset. On macOS use `~/Library/Application Support/advent-calendar`; on Windows use `%APPDATA%\\advent-calendar`.
- OAuth access and refresh tokens are stored in the operating system credential store (macOS Keychain, Windows Credential Manager, or Linux Secret Service). Linux users need an available Secret Service provider. OAuth config files and service-account keys must not be committed.

## Templates

- The app ships with a built-in starter profile based on `profile.example.json`.
- On first setup or first use, the starter profile is copied to `<config-dir>/templates/default.json` only if the user has not already created it.
- `month-init --template FILE` uses the specified file. If `--template` is omitted, it uses the user's `templates/default.json`.
- User edits to templates are preserved across app upgrades.
- A profile contains a `tasks` array. Each task has a unique `name`, optional `emoji`, `color` in `#RRGGBB` format, and `weekly_target` from 0 to 7.

## CLI Workflows

- `advent-calendar setup --spreadsheet-id ID` saves the selected spreadsheet ID and uses OAuth by default.
- `advent-calendar setup --spreadsheet-id ID --service-account-key FILE` saves service-account configuration.
- `advent-calendar reset-settings` confirms before deleting the user's configuration directory and stored OAuth tokens; `--yes` skips confirmation.
- `advent-calendar show [--week YYYY-MM-DD]` displays the Monday-through-Sunday week containing the date. It defaults to the current local week.
- `advent-calendar complete --task TASK [--date YYYY-MM-DD] [--uncheck]` checks a task for today or the selected date. Task matching is case-insensitive and accepts a task label with or without its trailing emoji. Missing and ambiguous matches fail without writing.
- `advent-calendar month-init [--month YYYY-MM] [--template FILE]` initializes a new tab. It defaults to the current local month and the user's default template. It refuses to replace an existing tab.
- `advent-calendar --help` documents commands and options.
- Errors are concise by default; the global `-v`/`--verbose` flag prints the full error chain.

## Spreadsheet Layout and Behavior

- Tabs use English `Month-Year` names, such as `October-2026`.
- Column A contains bold task labels. Columns B:H are Monday through Sunday and are 87 pixels wide on new tabs. In-month task cells use their task's profile color; unused dates remain uncolored.
- The weekday header row and task-name column are frozen. Native Boolean checkboxes are added only for dates in the month.
- Column I contains a centered `🎉🎉🎉` marker when that task's weekly completion target is met. Its background becomes light green while the marker is shown.
- `show` displays aligned task and date columns, with terminal-aware status colors and symbols. Non-terminal output has no ANSI color codes.
- `complete` reads the sheet, finds exactly one date/task cell, and updates only that checkbox.
- `month-init` checks for an existing tab before adding it, writes the calendar layout, and applies formatting.

## Acceptance Criteria

- A local Cargo install produces an `advent-calendar` executable.
- Setup saves configuration and creates the default template without overwriting user files.
- OAuth and service-account authentication both work from saved configuration and environment overrides.
- A user can view the current or selected week, update a unique task, and initialize a month without changing existing tabs.
- Invalid configuration, inaccessible sheets, missing tasks, and ambiguous matches return actionable errors.
- No credential contents are written to project configuration or source control.

## Implementation Notes

- Rust uses the 2024 edition. The Sheets API client provides API access; the OAuth module implements the PKCE authorization flow and refresh.
- Existing spreadsheets must use the month tab naming and weekly row layout above for `show` and `complete` to locate cells.
- Google OAuth consent and refresh-token policies apply to OAuth client deployments. Service-account keys should be used only when an app identity is needed and protected as credentials.
