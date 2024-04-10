use std::{
    io::{stdout, Write},
    time::Duration,
};

use solana_client::{
    client_error::{ClientError, ClientErrorKind, Result as ClientResult},
    nonblocking::rpc_client::RpcClient,
    rpc_config::RpcSendTransactionConfig,
};
use solana_program::instruction::Instruction;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    signature::{Signature, Signer},
    transaction::Transaction,
};
use solana_transaction_status::{TransactionConfirmationStatus, UiTransactionEncoding};

use crate::Miner;

use futures::Future;

const RPC_RETRIES: usize = 1;

impl Miner {
    pub async fn send_and_confirm(
        &self,
        ixs: &[Instruction],
        skip_confirm: bool,
    ) -> ClientResult<Signature> {
        const CONFIRM_RETRIES: usize = 3;
        const GATEWAY_RETRIES: usize = 40;
        const CONFIRM_INTERVAL: usize = 4;

        self.send_and_confirm_2(ixs, skip_confirm, CONFIRM_RETRIES, CONFIRM_INTERVAL, GATEWAY_RETRIES).await
    }

    pub async fn send_and_confirm_2(
        &self,
        ixs: &[Instruction],
        skip_confirm: bool,
        confirm_retries: usize,
        confirm_interval: usize,
        gateway_retries: usize,
    ) -> ClientResult<Signature> {
        let get_ixs = || async { ixs.to_vec() };
        self.send_and_confirm_3(
            get_ixs, 
            skip_confirm, 
            confirm_retries, 
            confirm_interval,
            gateway_retries
        ).await
    }

    pub async fn send_and_confirm_3<'a, Fut>(
        &self,
        get_ixs: impl Fn() -> Fut,
        skip_confirm: bool,
        confirm_retries: usize,
        confirm_interval: usize,
        gateway_retries: usize,
    ) -> ClientResult<Signature> 
    where 
        Fut: Future<Output = Vec<Instruction>>,
    {
        let mut stdout = stdout();
        let signer = self.signer();
        let client =
            RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::confirmed());

        // Return error if balance is zero
        let balance = client
            .get_balance_with_commitment(&signer.pubkey(), CommitmentConfig::confirmed())
            .await
            .unwrap();
        if balance.value <= 500000 { // 0.0005 SOL
            panic!("Too low balance");
            // return Err(ClientError {
            //     request: None,
            //     kind: ClientErrorKind::Custom("Insufficient SOL balance".into()),
            // });
        }

        // Submit tx
        let mut sigs = vec![];
        let mut attempts = 0;
        loop {
            println!("Attempt: {:?}", attempts);
            let (hash, slot) = client
                .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                .await
                .unwrap(); // TODO add retry on error (timeout happens here sometimes which leads to crash of worker)
            let send_cfg = RpcSendTransactionConfig {
                skip_preflight: true,
                preflight_commitment: Some(CommitmentLevel::Confirmed),
                encoding: Some(UiTransactionEncoding::Base64),
                max_retries: Some(RPC_RETRIES),
                min_context_slot: Some(slot),
            };
            let ixs = get_ixs().await;
            let mut tx = Transaction::new_with_payer(&ixs, Some(&signer.pubkey()));
            tx.sign(&[&signer], hash);

            match client.send_transaction_with_config(&tx, send_cfg).await {
                Ok(sig) => {
                    sigs.push(sig);
                    println!("{:?}", sig);

                    // Confirm tx
                    if skip_confirm {
                        return Ok(sig);
                    }
                    for _ in 0..confirm_retries {
                        let duration_millis = confirm_interval * 1000;
                        std::thread::sleep(Duration::from_millis(duration_millis as u64));
                        match client.get_signature_statuses(&sigs).await {
                            Ok(signature_statuses) => {
                                println!("Confirms: {:?}", signature_statuses.value);
                                for signature_status in signature_statuses.value {
                                    if let Some(signature_status) = signature_status.as_ref() {
                                        if signature_status.confirmation_status.is_some() {
                                            let current_commitment = signature_status
                                                .confirmation_status
                                                .as_ref()
                                                .unwrap();
                                            match current_commitment {
                                                TransactionConfirmationStatus::Processed => {}
                                                TransactionConfirmationStatus::Confirmed
                                                | TransactionConfirmationStatus::Finalized => {
                                                    println!("Transaction landed!");
                                                    return Ok(sig);
                                                }
                                            }
                                        } else {
                                            println!("No status");
                                        }
                                    }
                                }
                            }

                            // Handle confirmation errors
                            Err(err) => {
                                println!("Error: {:?}", err);
                            }
                        }
                    }
                    println!("Transaction did not land");
                }

                // Handle submit errors
                Err(err) => {
                    println!("Error {:?}", err);
                }
            }
            stdout.flush().ok();

            // Retry
            std::thread::sleep(Duration::from_millis(200));
            attempts += 1;
            if attempts > gateway_retries {
                return Err(ClientError {
                    request: None,
                    kind: ClientErrorKind::Custom("Max retries".into()),
                });
            }
        }
    }
}
