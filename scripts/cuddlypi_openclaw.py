"""SSH tunnel to Raspberry Pi for OpenClaw on port 18789.
Auto-reconnects on disconnect. Opens browser on first run.
"""
import subprocess
import sys
import time
import webbrowser

HOST = "cuddly-pi@82.65.33.206"
PASSWORD = "Cwctch228!"
HOSTKEY = "SHA256:EyO+RTijGkFSPSWWSgrKjyQU7WA+MAxhqv43Tk4zBNQ"
PLINK = r"C:\Program Files\PuTTY\plink.exe"
PORT = 18789
TOKEN_URL = f"http://127.0.0.1:{PORT}/#token=6ea6fbef2c4a11c46a280762fcdbcc35c1581134ddf146ba"

first_run = True

while True:
    ts = time.strftime("%H:%M:%S")
    print(f"[{ts}] Подключение к OpenClaw (порт {PORT})...")
    sys.stdout.flush()

    if first_run:
        webbrowser.open(TOKEN_URL)
        first_run = False

    result = subprocess.run(
        [
            PLINK, "-ssh", HOST,
            "-pw", PASSWORD,
            "-N",
            "-L", f"{PORT}:127.0.0.1:{PORT}",
            "-batch",
            "-hostkey", HOSTKEY,
        ]
    )

    ts = time.strftime("%H:%M:%S")
    print(f"[{ts}] Соединение разорвано (код: {result.returncode}). Переподключение через 5 секунд...")
    sys.stdout.flush()
    time.sleep(5)
