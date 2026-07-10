# Getting Started

## What HELM is for

HELM is a local dashboard for services and scripts that you run on your own machine or server. Use it when you want one place to start tools, stop them, check logs, and keep small maintenance jobs organized.

## Open the dashboard

Open `http://127.0.0.1:7010` in your browser.

If a PIN is enabled, enter it on the login screen. If there is no PIN prompt, authentication is disabled for this HELM instance.

## First setup

1. Open **Services** if you want to manage a long-running process.
2. Open **Scripts** if you want to run a one-time or scheduled command.
3. Add tags and descriptions so entries stay easy to scan later.
4. Start one item and check its logs before adding more.

## Services vs scripts

- **Services** - long-running processes such as bots, web servers, workers, local APIs, and file watchers.
- **Scripts** - one-time commands such as backups, sync jobs, cleanup tasks, reports, or maintenance checks.

## Good defaults

- Use clear names like `Discord Bot`, `Backup Database`, or `Local API`.
- Set the working directory when the command depends on relative paths.
- Add tags such as `bot`, `backup`, `dev`, `prod`, or `maintenance`.
- Add a health check URL for web services that expose one.
