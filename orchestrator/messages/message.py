from pydantic import BaseModel

class MessageAdditionalInfo:
  
  def __init__(self, worker_name: str):
    self.worker_name = worker_name

class Message(BaseModel):
  msg_type: str
  
  def valid_data(self) -> bool:
    return True
  
  def handle(self, info: MessageAdditionalInfo):
    pass