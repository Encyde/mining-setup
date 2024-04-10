from worker import Worker
from config import parse_config
import asyncio
import sys

async def run_delayed(worker: Worker, index: int):
  await asyncio.sleep(index)
  await worker.run()

async def main():
  config_path = sys.argv[1]
  
  config = parse_config(config_path)
  print(f"Launching with config:\n{config}")
  commands = config.build_commands()
  
  workers = [Worker(command, config.worker_name(i), config.logs_dir) for i, command in enumerate(commands)]
  await asyncio.gather(*[
    run_delayed(worker, i) for i, worker in enumerate(workers)
  ])
  print("All subprocesses have completed.")

if __name__ == "__main__":
  asyncio.run(main())
