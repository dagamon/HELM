# Troubleshooting

## A service does not start

- Check the command path.
- Check the working directory.
- Check the arguments.
- Check environment variables.
- Open the logs and read the newest stderr output.
- If the service depends on another service, make sure the dependency is running.

## A script works in my terminal but fails in HELM

The HELM service may run with a different user, environment, or PATH than your terminal.

- Use full paths for important executables.
- Set the working directory explicitly.
- Add required environment variables to the script configuration.
- Avoid commands that require interactive input.

## A Python command cannot find packages

Use the Python executable from the correct virtual environment, or set the service virtual environment path when configuring a Python service.

## A Rust service cannot find Cargo

If HELM runs as a background service, Cargo may not be available in its PATH. Use a built binary path, or configure the environment so HELM can find Cargo.

## Logs are empty

- Confirm the process actually started.
- Confirm the command writes to stdout or stderr.
- For scripts, check the run history.
- For services, restart the service from the dashboard and watch live logs.

## The dashboard does not load

- Confirm HELM is running.
- Open `http://127.0.0.1:7010`.
- If a PIN is enabled, confirm you are using the correct PIN.
- If the browser is showing an old screen, refresh the page.
