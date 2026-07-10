# Logs and Status

## Service status

Service cards show whether a service is running, stopped, crashed, healthy, or unhealthy. If a service has a health check URL, HELM uses that check in addition to the process state.

## Open logs

Open a service or script log view to inspect recent output. Logs include stdout and stderr so you can see normal output and errors in one place.

## Live logs

When a process is running, the log viewer follows new output in real time. Use this while starting a service or running a script to catch errors immediately.

## Copy logs

Use the copy button in the log viewer to copy the visible log output. This is useful when sending an error to another person or saving a quick note for later.

## What to check first

1. Look at the newest stderr lines.
2. Check whether the command path is correct.
3. Check whether the working directory is correct.
4. Check whether required environment variables are present.
5. If a health check is failing, open the health check URL manually.

## Log retention

HELM keeps recent logs and periodically removes old stored output. If you need a permanent record, copy the relevant output before it ages out.
