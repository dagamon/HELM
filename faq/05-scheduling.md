# Scheduling Scripts

## What scheduling does

Scheduling lets HELM run scripts automatically using a cron expression. Use it for backups, cleanup jobs, sync tasks, reports, and regular health checks.

## Enable a schedule

1. Open **Scripts**.
2. Edit a script.
3. Enable the cron schedule.
4. Enter the schedule.
5. Save the script.
6. Check the next run time in the scripts page.

## Common schedules

- Every hour: `0 0 * * * *`
- Every day at 03:00: `0 0 3 * * *`
- Every Monday at 09:30: `0 30 9 * * Mon`
- Every 15 minutes: `0 */15 * * * *`

## Before enabling automation

- Run the script manually once.
- Confirm the logs look correct.
- Confirm the working directory is set.
- Confirm the script can run without interactive prompts.
- Confirm the command is safe to repeat.

## Failed scheduled runs

If a scheduled script fails, open the run history and inspect stderr. Most failures come from missing paths, missing environment variables, permissions, or commands that work in an interactive terminal but not in the HELM service environment.
