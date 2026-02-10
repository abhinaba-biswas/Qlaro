# Crux Cli

Crux is a lightweight, keyboard-centric terminal tool(Cli Version of CRUX) built in Go using the Bubble Tea framework. It is designed to handle large datasets efficiently without consuming excessive RAM. Crux features infinite scrolling and a modular architecture that stays out of your way.

## Key Features

- **Infinite Scroll (Batch Loading):** Crux loads data in chunks, allowing you to start browsing immediately without waiting for the entire file to parse. Large files remain on disk until you scroll down.

- **Row Inspector:** Terminal columns are narrow. Press Enter to open a detailed view of every field in the selected row.

- **Instant Filtering:** Use `/filter` to drill down into your data. It scans the loaded rows in real-time to find exactly what you need.

- **Dynamic Sorting:** Toggle between ascending and descending order on the fly to spot trends or outliers.

- **Zero-Config Export:** After filtering or sorting, export your current view into a new CSV using `/export`.

## Controls

### Navigation

- `j` / `k` (or arrow keys): Move the selection up and down.
- `/`: Open the command bar for quick actions.
- `Esc`: Toggle focus between the table and the command bar.
- `Enter`: Open the Row Inspector for the highlighted row.
- `s`: Sort the current view.
- `Ctrl + C`: Quit.

### Slash Commands

Type these into the command bar:

- `/load <filename.csv>` — Open a dataset.
- `/filter <keyword>` — Hide everything that does not match the keyword.
- `/reset` — Clear filters and return to the full dataset.
- `/export <new_file.csv>` — Save the current filtered/sorted view to a new CSV file.
