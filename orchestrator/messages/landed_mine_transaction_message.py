from messages.message import Message, MessageAdditionalInfo
from messages.timestamps_handler import TimestampsHandler

class LandedMineTransactionMessage(Message):
  def valid_data(self) -> bool:
    return self.msg_type == "landed_mine_transaction"
  
  def handle(self, info: MessageAdditionalInfo):
    TimestampsHandler.shared().handleLandedTransaction(info)