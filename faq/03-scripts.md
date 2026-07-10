# Running Scripts

## When to add a script

Add a script for work that starts, does a job, and exits: backups, file cleanup, report generation, sync tasks, migrations, checks, and small utilities.

## Add a script

1. Go to **Scripts**.
2. Click **Add script**.
3. Enter the command, working directory, and arguments.
4. Choose the run mode.
5. Save the script.
6. Run it once and inspect the result.

## Run modes

- **Exec** - runs the command directly. Use this for normal executable commands.
- **Shell** - runs through the system shell. Use this when you need shell features such as chained commands or shell built-ins.

Prefer **Exec** when possible. Use **Shell** only when the command requires shell behavior.

## Arguments

Put stable command options in **Arguments** instead of packing everything into the command field. This keeps the command easier to edit and reduces quoting mistakes.

## Working directory

Set a working directory when your script reads or writes relative files. This is especially important for project scripts that expect to run from the project root.

## Run history

After a script runs, HELM stores the run status, start time, stop time, exit code, and output logs. Use this history to confirm whether scheduled jobs are completing successfully.
