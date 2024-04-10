use std::{
    io::{stdout, Write}, os::unix::thread, sync::{atomic::AtomicBool, Arc, Mutex}
};

use ore::{self, state::{Bus, Treasury}, BUS_ADDRESSES, BUS_COUNT, EPOCH_DURATION};
use rand::Rng;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::client_error::ClientError;

use solana_sdk::{
    commitment_config::CommitmentConfig, compute_budget::ComputeBudgetInstruction, instruction::Instruction, keccak::{hashv, Hash as KeccakHash}, signature::{Keypair, Signer},
    keccak::Hash
};

use crate::{
    cu_limits::{CU_LIMIT_MINE, CU_LIMIT_RESET}, dynamic_config::{self, DynamicConfig}, treasury, utils::{get_clock_account, get_proof, get_treasury}, Miner
};

use serde::{Serialize, Deserialize};

use rayon::prelude::*;

// Odds of being selected to submit a reset tx
const RESET_ODDS: u64 = 20;

impl Miner {
    pub async fn mine(&self, threads: u64, thread_pool: bool, dynamic_config: bool) {
        let num_global_threads = threads.try_into().unwrap();

        if thread_pool {
            rayon::ThreadPoolBuilder::new().num_threads(num_global_threads).build_global().unwrap();   
        }

        self.send_started_message();

        // Register, if needed.
        let signer = Arc::new(self.signer());
        self.register().await;
        let mut stdout = stdout();
        // let mut rng = rand::thread_rng();

        // Start mining loop
        loop {
            // Fetch account state
            let balance = self.get_ore_display_balance().await;
            let treasury = get_treasury(self.cluster.clone()).await;
            let proof = get_proof(self.cluster.clone(), signer.pubkey()).await;
            let rewards =
                (proof.claimable_rewards as f64) / (10f64.powf(ore::TOKEN_DECIMALS as f64));
            let reward_rate =
                (treasury.reward_rate as f64) / (10f64.powf(ore::TOKEN_DECIMALS as f64));
            stdout.write_all(b"\x1b[2J\x1b[3J\x1b[H").ok();
            println!("Balance: {} ORE", balance);
            println!("Claimable: {} ORE", rewards);
            println!("Reward rate: {} ORE", reward_rate);

            // Escape sequence that clears the screen and the scrollback buffer
            println!("\nMining for a valid hash...");
            let (next_hash, nonce) = if thread_pool {
                self.find_next_hash_par_2(proof.hash.into(), treasury.difficulty.into(), threads)
            } else {
                self.find_next_hash_par(proof.hash.into(), treasury.difficulty.into(), threads)
            };

            // Submit mine tx.
            // Use busses randomly so on each epoch, transactions don't pile on the same busses
            println!("\n\nSubmitting hash for validation...");
            loop {

                // Reset epoch, if needed
                let treasury = get_treasury(self.cluster.clone()).await;
                // let clock = get_clock_account(self.cluster.clone()).await;
                // let threshold = treasury.last_reset_at.saturating_add(EPOCH_DURATION);

                // if clock.unix_timestamp.ge(&threshold) {
                //     // There are a lot of miners right now, so randomly select into submitting tx
                //     if rng.gen_range(0..RESET_ODDS).eq(&0) {
                //         println!("Sending epoch reset transaction...");
                //         let cu_limit_ix =
                //             ComputeBudgetInstruction::set_compute_unit_limit(CU_LIMIT_RESET);
                //         let cu_price_ix =
                //             ComputeBudgetInstruction::set_compute_unit_price(priority_fee);
                //         let reset_ix = ore::instruction::reset(signer.pubkey());
                //         self.send_and_confirm_2(
                //             &[cu_limit_ix, cu_price_ix, reset_ix], 
                //             true,
                //             self.confirm_retries,
                //             self.gateway_retries,
                //         )
                //         .await
                //         .ok();
                //     }
                // }

                match self.send_and_confirm_3(
                    || async { 
                        self.build_instructions(
                            dynamic_config, 
                            signer.clone(), 
                            treasury,
                            next_hash, 
                            nonce
                        ).await
                    },
                    false,
                    self.confirm_retries,
                    self.confirm_interval,
                    self.gateway_retries,
                )
                .await
                {
                    Ok(sig) => {
                        println!("Success: {}", sig);
                        self.send_landed_mine_message();
                        break;
                    }
                    Err(_err) => {
                        // TODO
                        self.send_failed_transaction_message(_err);
                    }
                }
            }
        }
    }

    async fn build_instructions(
        &self, 
        dynamic_config: bool,
        signer: Arc<Keypair>, 
        treasury: Treasury, 
        next_hash: Hash, 
        nonce: u64
    ) -> Vec<Instruction> {
        let (bus, priority_fee) = if dynamic_config {
            let dynamic_config = self.get_dynamic_config().await;
            self.find_bus_and_priority_id(treasury.reward_rate, dynamic_config).await
        } else {
            self.default_find_bus_and_priority_id(treasury.reward_rate).await
        };
        let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(CU_LIMIT_MINE);
        let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(priority_fee);
        let ix_mine = ore::instruction::mine(
            signer.pubkey(),
            BUS_ADDRESSES[bus.id as usize],
            next_hash.into(),
            nonce,
        );
        let instructions = vec![cu_limit_ix, cu_price_ix, ix_mine];
        instructions
    }

    async fn find_bus_and_priority_id(&self, reward_rate: u64, dynamic_config: Option<DynamicConfig>) -> (Bus, u64) {
        if let Some(config) = dynamic_config {
            for suggested_bus in config.busses.iter() {
                if let Ok(bus) = self.get_bus(suggested_bus.id).await {
                    if bus.rewards.gt(&reward_rate.saturating_mul(4)) {
                        return (bus, suggested_bus.priority_fee);
                    }
                }
            }
        }
        
        self.default_find_bus_and_priority_id(reward_rate).await
    }

    async fn default_find_bus_and_priority_id(&self, reward_rate: u64) -> (Bus, u64) {
        let mut rng = rand::thread_rng();
        loop {
            let bus_id = rng.gen_range(0..BUS_COUNT);
            if let Ok(bus) = self.get_bus(bus_id).await {
                if bus.rewards.gt(&reward_rate.saturating_mul(4)) {
                    return (bus, self.priority_fee);
                }
            }
        }
    }

    fn find_next_hash_par(
        &self,
        hash: KeccakHash,
        difficulty: KeccakHash,
        threads: u64,
    ) -> (KeccakHash, u64) {
        let found_solution = Arc::new(AtomicBool::new(false));
        let solution = Arc::new(Mutex::<(KeccakHash, u64)>::new((
            KeccakHash::new_from_array([0; 32]),
            0,
        )));
        let signer = self.signer();
        let pubkey = signer.pubkey();
        let thread_handles: Vec<_> = (0..threads)
            .map(|i| {
                std::thread::spawn({
                    let found_solution = found_solution.clone();
                    let solution = solution.clone();
                    // let mut stdout = stdout();
                    move || {
                        let n = u64::MAX.saturating_div(threads).saturating_mul(i);
                        let mut next_hash: KeccakHash;
                        let mut nonce: u64 = n;
                        loop {
                            next_hash = hashv(&[
                                hash.to_bytes().as_slice(),
                                pubkey.to_bytes().as_slice(),
                                nonce.to_le_bytes().as_slice(),
                            ]);
                            if nonce % 10_000 == 0 {
                                if found_solution.load(std::sync::atomic::Ordering::Relaxed) {
                                    return;
                                }
                                // if n == 0 {
                                //     stdout
                                //         .write_all(
                                //             format!("\r{}", next_hash.to_string()).as_bytes(),
                                //         )
                                //         .ok();
                                // }
                            }
                            if next_hash.le(&difficulty) {
                                // stdout
                                //     .write_all(format!("\r{}", next_hash.to_string()).as_bytes())
                                //     .ok();
                                found_solution.store(true, std::sync::atomic::Ordering::Relaxed);
                                let mut w_solution = solution.lock().expect("failed to lock mutex");
                                *w_solution = (next_hash, nonce);
                                return;
                            }
                            nonce += 1;
                        }
                    }
                })
            })
            .collect();

        for thread_handle in thread_handles {
            thread_handle.join().unwrap();
        }

        let r_solution = solution.lock().expect("Failed to get lock");
        *r_solution
    }

    fn find_next_hash_par_2(
        &self,
        hash: KeccakHash,
        difficulty: KeccakHash,
        threads: u64,
    ) -> (KeccakHash, u64) {
        let found_solution = Arc::new(AtomicBool::new(false));
        let solution = Arc::new(Mutex::<(KeccakHash, u64)>::new((
            KeccakHash::new_from_array([0; 32]),
            0,
        )));
        let signer = self.signer();
        let pubkey = signer.pubkey();

        let thread_numbers: Vec<u64> = (0..threads).collect();
        
        thread_numbers.par_iter()
            .for_each(|&i| {
                let found_solution = found_solution.clone();
                let solution = solution.clone();
                let n = u64::MAX.saturating_div(threads).saturating_mul(i);
                let mut next_hash: KeccakHash;
                let mut nonce: u64 = n;
                loop {
                    next_hash = hashv(&[
                        hash.to_bytes().as_slice(),
                        pubkey.to_bytes().as_slice(),
                        nonce.to_le_bytes().as_slice(),
                    ]);
                    if nonce % 10_000 == 0 {
                        if found_solution.load(std::sync::atomic::Ordering::Relaxed) {
                            return;
                        }
                    }
                    if next_hash.le(&difficulty) {
                        found_solution.store(true, std::sync::atomic::Ordering::Relaxed);
                        let mut w_solution = solution.lock().expect("failed to lock mutex");
                        *w_solution = (next_hash, nonce);
                        return;
                    }
                    nonce += 1;
                }
            });

        let r_solution = solution.lock().expect("Failed to get lock");
        *r_solution
    }

    pub async fn get_ore_display_balance(&self) -> String {
        let client =
            RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::confirmed());
        let signer = self.signer();
        let token_account_address = spl_associated_token_account::get_associated_token_address(
            &signer.pubkey(),
            &ore::MINT_ADDRESS,
        );
        match client.get_token_account(&token_account_address).await {
            Ok(token_account) => {
                if let Some(token_account) = token_account {
                    token_account.token_amount.ui_amount_string
                } else {
                    "0.00".to_string()
                }
            }
            Err(_) => "Err".to_string(),
        }
    }

    fn send_started_message(&self) {
        let message = StartedMessage::new();
        let json = serde_json::to_string(&message).unwrap();
        println!("{}", json);
    }

    fn send_landed_mine_message(&self) {
        let message = LandedMineMessage::new();
        let json = serde_json::to_string(&message).unwrap();
        println!("{}", json);
    }

    fn send_failed_transaction_message(&self, err: ClientError) {
        let message = FailedTransactionMessage::new(err);
        let json = serde_json::to_string(&message).unwrap();
        println!("{}", json);
    }
}

#[derive(Serialize, Deserialize)]
struct StartedMessage {
    msg_type: String
}

impl StartedMessage {
    fn new() -> Self {
        Self {
            msg_type: "started".to_string()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct LandedMineMessage {
    msg_type: String,
}

impl LandedMineMessage {
    fn new() -> Self {
        Self {
            msg_type: "landed_mine_transaction".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct FailedTransactionMessage {
    msg_type: String,
    error: String
}

impl FailedTransactionMessage {
    fn new(error: ClientError) -> Self {
        Self {
            msg_type: "failed_transaction".to_string(),
            error: error.to_string()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SearchHadhStart {
    msg_type: String,
    error: String
}

impl SearchHadhStart {
    fn new(error: ClientError) -> Self {
        Self {
            msg_type: "failed_transaction".to_string(),
            error: error.to_string()
        }
    }
}