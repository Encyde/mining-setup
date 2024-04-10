from messages.message import Message, MessageAdditionalInfo

class FailedTransactionMessage(Message):
  error: str
  
  def valid_data(self) -> bool:
    return self.msg_type == "failed_transaction"
  
  def handle(self, info: MessageAdditionalInfo):
    pass
    # print(f"{info.worker_name} failed transaction: {self.error}")