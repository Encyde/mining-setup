import json
import messages.message
from messages.started_message import StartedMessage
from messages.landed_mine_transaction_message import LandedMineTransactionMessage
from messages.failed_transaction_message import FailedTransactionMessage
    
def parse_message(data: bytes | str) -> messages.message.Message | None:
  all_messages = [
    StartedMessage, 
    LandedMineTransactionMessage,
    FailedTransactionMessage
  ]
  
  for message in all_messages:
    try:
      js = json.loads(data)
      message_obj = message(**js)
      if message_obj.valid_data():
        return message_obj
    except:
      continue
    
  return None