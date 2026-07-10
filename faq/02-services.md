# Managing Services

## When to add a service

Add a service for anything that should keep running until you stop it: a bot, backend, frontend dev server, queue worker, tunnel, watcher, or local utility.

## Add a service

1. Go to **Services**.
2. Click **Add service**.
3. Fill in the name, type, command, and working directory.
4. Add arguments and environment variables if the command needs them.
5. Save the service.
6. Start it and open the log view to confirm it runs correctly.

## Common service fields

- **Name** - the label shown in the dashboard.
- **Description** - short context for what the service does.
- **Command** - the executable or command to run.
- **Working directory** - where the command should run from.
- **Arguments** - command-line arguments passed after the command.
- **Environment** - custom variables available only to this service.
- **URL** - a link you can open from the service card.
- **Health check URL** - an HTTP endpoint HELM can probe.

## Auto start and restart

- Enable **Auto start** when the service should start with HELM.
- Enable **Restart on crash** when a process should be relaunched automatically after an unexpected exit.

Use restart-on-crash for reliable background processes. Leave it off for commands that are expected to exit on their own.

## Dependencies

If one service needs another service first, add it as a dependency. HELM will block the dependent service from starting until the required service is running.

## Rust services

For Rust projects, use the Rust service type when you want HELM to run a Cargo project or a built binary. Use the manifest path for Cargo-based runs, or a binary path when you already have a compiled executable.
