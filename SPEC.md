# Google Sheets Advent Calendar CLI — Specification

## Purpose

Provide a command-line interface for viewing and updating an existing Google Sheets calendar of daily goals. The CLI should also create month-specific sheets from task profile templates. This document describes the initial MVP behavior and its layout assumptions.

## Users and Core Workflows

1. **View progress:** retrieve the current calendar state and display days, tasks, and completion status in a readable terminal format.
2. **Complete a goal:** identify a date and task; check or uncheck that goal in the existing spreadsheet; report success or a useful error.
3. **Create a month:** create a new month sheet from a task profile template without changing prior months.

## Proposed CLI

- `advent-calendar show [--week YYYY-MM-DD]` — display the current Monday–Sunday week by default; an explicit date selects its week.
- `advent-calendar complete --date YYYY-MM-DD --task TASK [--uncheck]` — check the task for that date, or clear it with `--uncheck`. Reject missing and ambiguous matches.
- `advent-calendar create-month --month YYYY-MM --template FILE` — create a month tab from a JSON template containing task names, emojis, colors, and weekly targets.
- `advent-calendar --help` — document commands, examples, and configuration.

## Observed Sheet Layout

The provided screenshot shows a month calendar arranged as a grid:

- Column A contains six colored labels: “Acarizax 💊”, “Physical 🏃”, “Cooking 🍲”, “Sara ❤️”, “Meditation 🧘”, and “Study 🧠”. They appear to be goal/task categories, but whether any label is a profile needs confirmation.
- Columns B through H represent Monday through Sunday. Each weekly block has a date row followed by one row per label. Dates appear as day and month (for example, `31.8.`); checkboxes at their intersections show completion.
- Weekly blocks are stacked vertically. Some weekday cells are blank at the start or end of the month. The screenshot does not show the tab name or year.
- Column K contains celebration emoji indicating whether the weekly completion target was met. The target comes from the profile template. Columns I and J appear hidden; the CLI should leave them untouched unless their purpose is established.

Profiles are templates specifying task names, emojis, background colors, and each task's weekly completion target. Months are separate tabs named in English `Month-Year` format (for example, `October-2026`). New tabs are generated from the supplied profile template. The MVP expects existing tabs to use this naming and the visible weekly grid layout.

## Google Sheets Integration

- Connect to the existing spreadsheet using the Google Sheets API and a service account JSON key supplied locally. Share the spreadsheet with the service account and grant edit access; configure the spreadsheet ID and key path outside source control.
- Read the sheet before updates, verify the requested date/task mapping, and write only the relevant completion cell.
- For month creation, check for an existing month sheet before creating one. Require an explicit overwrite strategy if a sheet already exists; never replace it silently.
- Show actionable errors for missing configuration, inaccessible sheets, absent rows/tasks, duplicate matches, and API/network failures. Do not report success unless the write is confirmed.

## Display and Data Behavior

Display should make the selected week, dates, goal labels, and completion state clear. By default, select the current week using the machine's local date. Preserve the spreadsheet's existing organization and values. Use native checkboxes and support unchecking. A `🎉` marker in column K indicates that a task's weekly target from the template has been reached.

## Acceptance Criteria

- A user can view the existing sheet's state for the current week or a specified week.
- A user can complete an existing goal and see the corresponding change reflected on a subsequent read.
- A user can create a month sheet for new profiles and tasks without modifying other months.
- Invalid and ambiguous requests fail clearly and do not write unintended cells.
- Credentials and spreadsheet identifiers are not committed to the repository.

## MVP Notes

- The Rust `google-sheets4` client handles the Sheets API; its `yup-oauth2` service-account authenticator handles local JSON credentials.
- The MVP template format is documented in `README.md` and illustrated by `profile.example.json`.
- Existing workbook locale and exact cell-value representation may differ from the screenshot. The MVP recognizes the displayed day-month date format and `Month-Year` tab convention; validate against the actual sheet before relying on writes.
