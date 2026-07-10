# Backup and Restore

## What export includes

Export saves the configured services and scripts to a JSON file. Use it before large edits, moving HELM to another machine, or reinstalling the dashboard.

## What export does not include

Export does not save live process state. A service that was running before export will not automatically be running after import unless its configuration uses auto start and HELM starts it later.

## Create a backup

1. Open the export action in HELM or use the CLI if you prefer terminal workflows.
2. Save the JSON file somewhere outside the HELM folder.
3. Keep a recent copy before making major changes.

## Restore from a backup

1. Open the import action.
2. Select the JSON file.
3. Review the services and scripts after import.
4. Start important services manually and check their logs.

## Moving to another machine

After importing on a new machine, check paths carefully. Commands, working directories, virtual environments, binaries, and project folders may be different on the new system.
