import json
import math
import os
from pathlib import Path
from datetime import datetime

class Config:
  
  def __init__(
    self, 
    ore_bin: str, 
    confirm_retries: int,
    confirm_interval: int,
    gateway_retries: int,
    keypairs_dir: str, 
    priority_fee_per_unit_mc_lamports: int, 
    rpcs: list[str],
    fallback_rpc: str,
    logs_dir: str,
    thread_pool: bool,
    dynamic_config: bool
  ):
    self.ore_bin = ore_bin
    self.confirm_retries = confirm_retries
    self.confirm_interval = confirm_interval
    self.gateway_retries = gateway_retries
    self.priority_fee_per_unit_mc_lamports = priority_fee_per_unit_mc_lamports
    self.rpcs = rpcs
    self.fallback_rpc = fallback_rpc
    self.thread_pool = thread_pool
    self.dynamic_config = dynamic_config
    
    logs_dir = os.path.join(logs_dir, datetime.now().isoformat())
    os.makedirs(logs_dir, exist_ok=True)
    self.logs_dir = logs_dir
    
    directory = Path(keypairs_dir)
    self.keypairs_dir = keypairs_dir
    
    keypairs = [f.name for f in directory.iterdir() if f.is_file()]
    self.keypairs = sorted(keypairs, key=lambda x: int(x.split("_")[2].split(".")[0]))
    
  def worker_name(self, index: int) -> str:
    return self.keypairs[index].split(".")[0]
    
  def build_commands(self) -> list[list[str]]:
    accounts_per_rpc = math.ceil(len(self.keypairs) / len(self.rpcs))
    threads = str(os.cpu_count())
    commands: list[list[str]] = []
    for i, keypair in enumerate(self.keypairs):
      keypair_path = os.path.join(self.keypairs_dir, keypair)
      command = [
        self.ore_bin,
        "--keypair", keypair_path,
        "--rpc", self.rpcs[i // accounts_per_rpc],
        "--priority-fee", str(self.priority_fee_per_unit_mc_lamports),
        "--confirm-retries", str(self.confirm_retries),
        "--confirm-interval", str(self.confirm_interval),
        "--gateway-retries", str(self.gateway_retries),
        "mine",
        "--threads", threads,
      ]
      if self.thread_pool:
        command.append("--thread-pool")
      if self.dynamic_config:
        command.append("--dynamic-config")
      commands.append(command)
      
    return commands
  
  def __str__(self) -> str:
    data = {
      "ore_bin": self.ore_bin, 
      "confirm_retries": self.confirm_retries,
      "confirm_interval": self.confirm_interval,
      "gateway_retries": self.gateway_retries,
      "keypairs": self.keypairs, 
      "priority_fee_per_unit_mc_lamports": self.priority_fee_per_unit_mc_lamports, 
      "rpcs": self.rpcs,
      "fallback_rpc": self.fallback_rpc,
      "logs_dir": self.logs_dir,
      "thread_pool": self.thread_pool,
      "dynamic_config": self.dynamic_config
    }
    pretty_json = json.dumps(data, indent=2, sort_keys=True)
    return pretty_json
  
  def __repr__(self) -> str:
    return str(self)
    
def parse_config(path: str) -> Config:
  with open(path, "r") as file:
    json_string = file.read()
    return Config(**json.loads(json_string))