use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    pubkey::Pubkey,
};
use std::sync::Arc;
use spl_token_client::{
    client::{ProgramRpcClient, ProgramRpcClientSendTransaction},
    token::{ExtensionInitializationParams, Token},
};
use std::time::Duration;

// Import our confidential ops directly
mod confidential_ops;

#[tokio::main]
async fn main() -> Result<()> {
    // Create connection to local test validator
    let rpc_url = "http://localhost:8899";
    println!("Connecting to Solana validator at {}", rpc_url);
    println!("Make sure validator is running with: solana-test-validator --clone-upgradeable-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb --url https://api.mainnet-beta.solana.com -r");
    
    let rpc_client = Arc::new(RpcClient::new_with_commitment_and_timeout(
        String::from(rpc_url),
        CommitmentConfig::confirmed(),
        Duration::from_secs(30),
    ));

    // Load keypair from default location
    let payer = load_keypair()?;
    println!("Using payer: {}", payer.pubkey());

    // Generate a new keypair for the mint
    let mint = Keypair::new();
    println!("Mint keypair generated: {}", mint.pubkey());

    // Number of decimals for the mint
    let decimals = 2;

    // TEST: Create a mint with confidential transfer extension
    println!("\n=== TEST: Create confidential mint ===");
    let (mint_pubkey, mint_sig) = confidential_ops::create_confidential_mint(
        rpc_client.clone(),
        &payer,
        &mint,
        decimals,
    ).await?;
    
    println!("Mint created with address: {}", mint_pubkey);
    println!("Transaction signature: {}", mint_sig);
    
    println!("\n=== Test completed successfully! ===");
    
    Ok(())
}

// Load the keypair from the default Solana CLI keypair path
fn load_keypair() -> Result<Keypair> {
    // Get the default keypair path
    let keypair_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".config/solana/id.json");

    // Read the keypair file
    let file = std::fs::File::open(&keypair_path)?;
    let keypair_bytes: Vec<u8> = serde_json::from_reader(file)?;

    // Create keypair from bytes
    let keypair = Keypair::from_bytes(&keypair_bytes)?;

    Ok(keypair)
} 