use anyhow::{anyhow, Context, Result};
use anchor_client::{Client, Cluster, ClientError};
use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    native_token::LAMPORTS_PER_SOL,
};
use serde::Deserialize;
use serde_yaml;
use std::{fs, rc::Rc, str::FromStr};

use sol_bank::accounts::{Initialize, Deposit, Withdraw};
use sol_bank::instruction::{Initialize as InitIx, Deposit as DepIx, Withdraw as WdrIx};
use sol_bank::UserAccount;

#[derive(Deserialize)]
struct Config {
    /// 64-byte keypair array
    keypair:     Vec<u8>,
    program_id:  String,
    network:     String, // "localnet" or "devnet"
}

fn main() -> Result<()> {
    // 1) Read and parse client_config.yaml
    let cfg: Config = serde_yaml::from_str(
        &fs::read_to_string("client_config.yaml")?
    ).context("Failed to read client_config.yaml")?;

    // 2) Reconstruct Keypair from bytes
    let raw = Keypair::from_bytes(&cfg.keypair)
        .map_err(|e| anyhow!("Failed to decode keypair bytes: {}", e))?;
    let payer = Rc::new(raw);

    // 3) Build the Anchor client and program handle
    let cluster = if cfg.network == "devnet" {
        Cluster::Devnet
    } else {
        Cluster::Localnet
    };
    let client  = Client::new(cluster, payer.clone());
    let program = client
        .program(Pubkey::from_str(&cfg.program_id)?)
        .context("Failed to load program client")?;

    // 4) Derive the PDA exactly as your on-chain code
    let (user_account, _bump) = Pubkey::find_program_address(
        &[b"user-account", payer.pubkey().as_ref()],
        &program.id(),
    );
    println!("➡️  PDA = {}", user_account);

    // 5) Check if the account already exists
    let already_init = match program.account::<UserAccount>(user_account) {
        Ok(existing) => {
            println!("🔎 account already initialized; balance = {}", existing.balance);
            true
        }
        Err(ClientError::AccountNotFound) => false,
        Err(e) => return Err(anyhow!("Failed to fetch account info: {}", e)),
    };

    // 6) Initialize (only if missing)
    if !already_init {
        let sig = program
            .request()
            .accounts(Initialize {
                user:           payer.pubkey(),
                user_account,
                system_program: system_program::ID,
            })
            .args(InitIx {})
            .send()
            .context("RPC initialize failed")?;
        println!("🔧 initialize tx = {}", sig);
    }

    // 7) Deposit 0.05 SOL
    let deposit_amount = (0.05 * LAMPORTS_PER_SOL as f64) as u64;
    let sig = program
        .request()
        .accounts(Deposit {
            user:           payer.pubkey(),
            user_account,
            system_program: system_program::ID,
        })
        .args(DepIx { amount: deposit_amount })
        .send()
        .context("RPC deposit failed")?;
    println!("➕ deposit tx     = {}", sig);

    // 8) Fetch & print on-chain balance
    let acct: UserAccount = program.account(user_account)?;
    println!("💰 balance now    = {} lamports", acct.balance);

    // 9) Withdraw 0.02 SOL
    let withdraw_amount = (0.02 * LAMPORTS_PER_SOL as f64) as u64;
    let sig = program
        .request()
        .accounts(Withdraw {
            user:           payer.pubkey(),
            user_account,
            system_program: system_program::ID,
        })
        .args(WdrIx { amount: withdraw_amount })
        .send()
        .context("RPC withdraw failed")?;
    println!("➖ withdraw tx    = {}", sig);

    // 10) Final balance
    let acct: UserAccount = program.account(user_account)?;
    println!("🏁 final balance = {} lamports", acct.balance);

    Ok(())
}
