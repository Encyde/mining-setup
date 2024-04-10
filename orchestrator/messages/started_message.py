from messages.message import Message, MessageAdditionalInfo
from messages.timestamps_handler import TimestampsHandler

class StartedMessage(Message):
  
  def valid_data(self) -> bool:
    return self.msg_type == "started"
  
  def handle(self, info: MessageAdditionalInfo):
    TimestampsHandler.shared().handleStarted(info)