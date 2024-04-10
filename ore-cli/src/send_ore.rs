use std::str::FromStr;

use solana_client::nonblocking::rpc_client::RpcClient;
use spl_token::state::Account as TokenAccount;

use solana_sdk::{
  commitment_config::CommitmentConfig, compute_budget::ComputeBudgetInstruction, program_pack::Pack, pubkey::Pubkey, signer::Signer
};

use crate::Miner;

impl Miner {

  pub async fn send_ore(&self, recipient_wallet: String) {
    let signer = self.signer();
    let client = RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::confirmed());

    println!("Sending ORE from {:?} to {}", signer.pubkey(), recipient_wallet);

    let signer_token_address = spl_associated_token_account::get_associated_token_address(
      &signer.pubkey(),
      &ore::MINT_ADDRESS,
    );
    let signer_account_data = client.get_account_data(&signer_token_address).await.unwrap();
    let signer_token_account = TokenAccount::unpack(&signer_account_data).unwrap();

    if signer_token_account.amount == 0 {
      println!("No ORE to send");
      return
    }

    let Ok(receiver_pubkey) = Pubkey::from_str(&recipient_wallet) else {
      println!("Invalid address: {:?}", recipient_wallet);
      return
    };

    let receiver_token_address = spl_associated_token_account::get_associated_token_address(
      &receiver_pubkey,
      &ore::MINT_ADDRESS,
    );
    println!("Sender token address: {:?}", signer_token_address);
    println!("Receiver token address: {:?}", receiver_token_address);

    let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(self.priority_fee);
    let transfer_ix = spl_token::instruction::transfer(
      &spl_token::id(),
      &signer_token_address,
      &receiver_token_address,
      &signer.pubkey(),
      &[&signer.pubkey()],
      signer_token_account.amount,
    ).unwrap();
    
    match self.send_and_confirm_2(
      &[cu_price_ix, transfer_ix], 
      false,
      self.confirm_retries,
      self.confirm_interval,
      self.gateway_retries
    ).await {
      Ok(_) => {
        println!("Sent ORE to {}", recipient_wallet);
      }
      Err(e) => {
        println!("Failed to send ORE to {}: {:?}", recipient_wallet, e);
      }
    };
  }
}