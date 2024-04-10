import subprocess
from datetime import datetime

def invoke_bash(command: str) -> str:
  output = subprocess.check_output(command.split(" "), text=True)
  return output.strip()

def log_message(message: str):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(timestamp, message)