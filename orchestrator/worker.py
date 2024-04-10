import os
import asyncio
from asyncio import StreamReader
import aiofiles
from aiofiles.threadpool.text import AsyncTextIOWrapper
from datetime import datetime
from messages.parse import parse_message
from messages.message import MessageAdditionalInfo

class Worker:
  def __init__(
    self, 
    command: list[str], 
    name: str,
    logs_dir_base: str
  ):
    self.command = command
    self.name = name
    self.logs_dir = os.path.join(logs_dir_base, name)
    self.timestamp: float | None = None
  
  async def run(self):
    os.makedirs(self.logs_dir, exist_ok=True)
    
    stdout_file_path = os.path.join(self.logs_dir, "stdout.log")
    stderr_file_path = os.path.join(self.logs_dir, "stderr.log")
    messages_file_path = os.path.join(self.logs_dir, "messages.log")
    self.timestamp = datetime.now().timestamp()
    
    process = await asyncio.create_subprocess_exec(
      *self.command, 
      limit = 1024 * 128, # 128 KiB
      stdout=asyncio.subprocess.PIPE, 
      stderr=asyncio.subprocess.PIPE
    )
    
    async with \
      aiofiles.open(stdout_file_path, "x") as stdout_file, \
      aiofiles.open(stderr_file_path, "x") as stderr_file, \
      aiofiles.open(messages_file_path, "x") as messages_file \
    :
      if process.stdout is None or process.stderr is None:
        raise Exception("Error starting the process: no stdout or stderr")
      
      # Create tasks for reading stdout and stderr
      tasks = [
        asyncio.create_task(self.read_out_stream(process.stdout, stdout_file, messages_file)),
        asyncio.create_task(self.read_err_stream(process.stderr, stderr_file))
      ]
      
      await asyncio.gather(*tasks)
      await process.wait()
        
      print(f"Process {self.name} ended with code {process.returncode}")
    
  def health_check_stdout_rate(self):
    if self.timestamp is None:
      raise Exception(f"{self.name} has no stdout timestamp")
    now = datetime.now().timestamp()

    # if now - self.timestamp > 5:
    #   print(f"{self.name}: no stdout for >5 seconds")
      
    self.timestamp = now
    
  async def read_out_stream(
    self, 
    stream: StreamReader, 
    file: AsyncTextIOWrapper,
    messages_file: AsyncTextIOWrapper
  ) -> None:
    try:
      while True:
        line = await stream.readline()
        if line:
          text = line.decode().strip()
          await file.write(self.date() + " " + text + "\n")
          await file.flush()
          self.health_check_stdout_rate()
          await self.handle_message(messages_file, text)
        else:
            break
    except:
      raise Exception(f"Error reading stdout for {self.name}")
            
  async def read_err_stream(self, stream: StreamReader, file: AsyncTextIOWrapper) -> None:
    try:
      while True:
        line = await stream.readline()
        if line:
          text = line.decode().strip()
          await file.write(self.date() + " " + text + "\n")
          await file.flush()
          print(f"{self.name} STDERR: {line}")
        else:
          break
    except:
      raise Exception(f"Error reading stderr for {self.name}")
    
  async def handle_message(self, messages_file: AsyncTextIOWrapper, text: str) -> None:
    message = parse_message(text)
    if message is None:
      return
    
    info = MessageAdditionalInfo(self.name)
    
    await messages_file.write(f"{self.name}: {message}\n")
    await messages_file.flush()
    message.handle(info)
    
  def date(self) -> str:
    return datetime.now().isoformat()