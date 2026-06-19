"""SSH connection to Raspberry Pi (cuddly-pi).
Uses Shell.Application to open a visible terminal window
even when running inside a Windows service (Session 0 isolation).
"""
import subprocess
import sys

HOST = "cuddly-pi@82.65.33.206"
PASSWORD = "Cwctch228!"
HOSTKEY = "SHA256:EyO+RTijGkFSPSWWSgrKjyQU7WA+MAxhqv43Tk4zBNQ"
PLINK = r"C:\Program Files\PuTTY\plink.exe"

print(f"Открытие SSH-терминала к {HOST}...")
sys.stdout.flush()

# Shell.Application.ShellExecute goes through explorer.exe (interactive user session).
# This makes the window visible on the desktop even from a Windows service (Session 0).
ps_cmd = (
    f"(New-Object -ComObject Shell.Application)"
    f".ShellExecute('cmd.exe', "
    f"'/k \"{PLINK}\" -pw {PASSWORD} -ssh {HOST} -hostkey {HOSTKEY}', "
    f"'', 'open', 1)"
)

subprocess.Popen(
    ["powershell.exe", "-NoProfile", "-WindowStyle", "Hidden", "-Command", ps_cmd],
    creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NO_WINDOW,
    stdin=subprocess.DEVNULL,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)

print("Терминал открыт.")
