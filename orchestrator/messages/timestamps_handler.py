from datetime import datetime
from messages.message import MessageAdditionalInfo
from utils import log_message

class TimestampsHandler:
  
  @staticmethod 
  def shared() -> "TimestampsHandler":
    return _timestampsHandler_shared
  
  def __init__(self) -> None:
    self.start_timestamps = dict[str, float]()
    self.mine_transaction_timestamps = dict[str, float]()
    
  def handleStarted(self, info: MessageAdditionalInfo):
    again = self.start_timestamps.get(info.worker_name) is not None
    print(f"{info.worker_name} launched mining{' again' if again else ''}")
    
    timestamp = datetime.now().timestamp()
    self.start_timestamps[info.worker_name] = timestamp
  
  def handleLandedTransaction(self, info: MessageAdditionalInfo):
    prev_timestamp = self.mine_transaction_timestamps.get(info.worker_name)
    timestamp = datetime.now().timestamp()
    if prev_timestamp is None:
      self.mine_transaction_timestamps[info.worker_name] = timestamp
      start_timestamp = self.start_timestamps.get(info.worker_name)
      if start_timestamp is None:
        raise Exception(f"{info.worker_name} landed a transaction without starting")
      diff = int(timestamp - start_timestamp)
      log_message(f"{info.worker_name} landed transaction in {diff} seconds")
    else:
      diff = int(timestamp - prev_timestamp)
      log_message(f"{info.worker_name} landed transaction in {diff} seconds")
      
    self.mine_transaction_timestamps[info.worker_name] = timestamp
    
  
    
_timestampsHandler_shared = TimestampsHandler()