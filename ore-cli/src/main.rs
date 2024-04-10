mod balance;
mod busses;
mod claim;
mod cu_limits;
#[cfg(feature = "admin")]
mod initialize;
mod mine;
mod register;
mod rewards;
mod send_and_confirm;
mod treasury;
mod send_ore;
#[cfg(feature = "admin")]
mod update_admin;
#[cfg(feature = "admin")]
mod update_difficulty;
mod utils;
mod dynamic_config;

use std::sync::Arc;

use clap::{command, Parser, Subcommand};
use solana_sdk::signature::{read_keypair_file, Keypair};

struct Miner {
    pub keypair_filepath: Option<String>,
    pub priority_fee: u64,
    pub cluster: String,
    pub confirm_retries: usize,
    pub confirm_interval: usize,
    pub gateway_retries: usize,
}

#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    #[arg(
        long,
        value_name = "NETWORK_URL",
        help = "Network address of your RPC provider",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    rpc: String,

    #[arg(
        long,
        value_name = "KEYPAIR_FILEPATH",
        help = "Filepath to keypair to use"
    )]
    keypair: Option<String>,

    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Number of microlamports to pay as priority fee per transaction",
        default_value = "0"
    )]
    priority_fee: u64,

    #[command(subcommand)]
    command: Commands,

    #[arg(
        long,
        value_name = "CONFIRM_RETRIES",
        help = "Confirm retries for a single transaction",
        default_value = "3"
    )]
    confirm_retries: usize,

    #[arg(
        long,
        value_name = "CONFIRM_INTERVAL",
        help = "Interval between confirm retries in seconds",
        default_value = "4"
    )]
    confirm_interval: usize,

    #[arg(
        long,
        value_name = "GETAWAY_RETRIES",
        help = "Retries for sending a set of instructions",
        default_value = "40"
    )]
    gateway_retries: usize,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Fetch the Ore balance of an account")]
    Balance(BalanceArgs),

    #[command(about = "Fetch the distributable rewards of the busses")]
    Busses(BussesArgs),

    #[command(about = "Register for mining")]
    Register(RegisterArgs),

    #[command(about = "Mine Ore using local compute")]
    Mine(MineArgs),

    #[command(about = "Claim available mining rewards")]
    Claim(ClaimArgs),

    #[command(about = "Send ORE to given wallet")]
    SendOre(SendOreArgs),

    #[command(about = "Register ORE associated token account")]
    RegisterToken(RegisterTokenArgs),

    #[command(about = "Fetch your balance of unclaimed mining rewards")]
    Rewards(RewardsArgs),

    #[command(about = "Fetch the treasury account and balance")]
    Treasury(TreasuryArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Initialize the program")]
    Initialize(InitializeArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Update the program admin authority")]
    UpdateAdmin(UpdateAdminArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Update the mining difficulty")]
    UpdateDifficulty(UpdateDifficultyArgs),
}

#[derive(Parser, Debug)]
struct BalanceArgs {
    #[arg(
        // long,
        value_name = "ADDRESS",
        help = "The address of the account to fetch the balance of"
    )]
    pub address: Option<String>,
}

#[derive(Parser, Debug)]
struct BussesArgs {}

#[derive(Parser, Debug)]
struct RewardsArgs {
    #[arg(
        // long,
        value_name = "ADDRESS",
        help = "The address of the account to fetch the rewards balance of"
    )]
    pub address: Option<String>,
}

#[derive(Parser, Debug)]
struct RegisterArgs {}

#[derive(Parser, Debug)]
struct MineArgs {
    #[arg(
        long,
        short,
        value_name = "THREAD_COUNT",
        help = "The number of threads to dedicate to mining",
        default_value = "1"
    )]
    threads: u64,

    #[arg(
        long,
        short,
        value_name = "THREAD_POOL",
        help = "Use thread pool for mining",
        default_value = "false"
    )]
    thread_pool: bool,

    #[arg(
        long,
        short,
        value_name = "DYNAMIC_CONFIG",
        help = "Use dynamic config from local server",
        default_value = "false"
    )]
    dynamic_config: bool,
}

#[derive(Parser, Debug)]
struct TreasuryArgs {}

#[derive(Parser, Debug)]
struct ClaimArgs {
    #[arg(
        // long,
        value_name = "AMOUNT",
        help = "The amount of rewards to claim. Defaults to max."
    )]
    amount: Option<f64>,

    #[arg(
        // long,
        value_name = "TOKEN_ACCOUNT_ADDRESS",
        help = "Token account to receive mining rewards."
    )]
    beneficiary: Option<String>,
}

#[derive(Parser, Debug)]
struct SendOreArgs {
    #[arg(
        value_name = "ORE_RECIPIENT_ADDRESS",
        help = "Wallet to which Ore is sent."
    )]
    recipient: String,
}

#[derive(Parser, Debug)]
struct RegisterTokenArgs {}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
struct InitializeArgs {}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
struct UpdateAdminArgs {
    new_admin: String,
}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
struct UpdateDifficultyArgs {}

#[tokio::main]
async fn main() {
    // Initialize miner.
    let args = Args::parse();
    let cluster = args.rpc;
    let miner = Arc::new(
        Miner::new(
            cluster.clone(), 
            args.priority_fee, 
            args.keypair,
            args.confirm_retries,
            args.confirm_interval,
            args.gateway_retries,
        )
    );

    // Execute user command.
    match args.command {
        Commands::Balance(args) => {
            miner.balance(args.address).await;
        }
        Commands::Busses(_) => {
            miner.busses().await;
        }
        Commands::Rewards(args) => {
            miner.rewards(args.address).await;
        }
        Commands::Treasury(_) => {
            miner.treasury().await;
        }
        Commands::Register(_) => {
            miner.register().await;
        }
        Commands::Mine(args) => {
            miner.mine(args.threads, args.thread_pool, args.dynamic_config).await;
        }
        Commands::Claim(args) => {
            miner.claim(cluster, args.beneficiary, args.amount).await;
        }
        Commands::SendOre(args) => {
            miner.send_ore(args.recipient).await;
        }
        Commands::RegisterToken(_) => {
            miner.register_token_account().await;
        }
        #[cfg(feature = "admin")]
        Commands::Initialize(_) => {
            miner.initialize().await;
        }
        #[cfg(feature = "admin")]
        Commands::UpdateAdmin(args) => {
            miner.update_admin(args.new_admin).await;
        }
        #[cfg(feature = "admin")]
        Commands::UpdateDifficulty(_) => {
            miner.update_difficulty().await;
        }
    }
}

impl Miner {
    pub fn new(
        cluster: String, 
        priority_fee: u64, 
        keypair_filepath: Option<String>,
        confirm_retries: usize,
        confirm_interval: usize,
        gateway_retries: usize,
    ) -> Self {
        Self {
            keypair_filepath,
            priority_fee,
            cluster,
            confirm_retries,
            confirm_interval,
            gateway_retries,
        }
    }

    pub fn signer(&self) -> Keypair {
        match self.keypair_filepath.clone() {
            Some(filepath) => read_keypair_file(filepath).unwrap(),
            None => panic!("No keypair provided"),
        }
    }
}
