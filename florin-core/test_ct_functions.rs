use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
};
use std::sync::Arc;

// Import our confidential ops module
use solana_confidential_mint::confidential_ops;

#[tokio::main]
async fn main() -> Result<()> {
    // Create connection to local test validator
    let rpc_url = "http://localhost:8899";
    println!("Connecting to Solana validator at {}", rpc_url);
    
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        String::from(rpc_url),
        CommitmentConfig::confirmed(),
    ));

    // Load keypair from default location
    let payer = load_keypair()?;
    println!("Using payer: {}", payer.pubkey());

    // Generate a new keypair for the mint
    let mint = Keypair::new();
    println!("Mint keypair generated: {}", mint.pubkey());

    // Number of decimals for the mint
    let decimals = 2;

    // STEP 1: Create a mint with confidential transfer extension
    println!("\n=== STEP 1: Create confidential mint ===");
    let (mint_pubkey, mint_sig) = confidential_ops::create_confidential_mint(
        rpc_client.clone(),
        &payer,
        &mint,
        decimals,
    ).await?;
    
    println!("Mint created with address: {}", mint_pubkey);
    println!("Transaction signature: {}", mint_sig);
    
    // STEP 2: Create sender token account with confidential transfer extension
    println!("\n=== STEP 2: Create confidential token account (sender) ===");
    let (sender_account, sender_elgamal_keypair, sender_ae_key, sig) = 
        confidential_ops::create_confidential_token_account(
            rpc_client.clone(),
            &payer,
            &mint_pubkey,
            &payer,
        ).await?;
    
    println!("Sender account created: {}", sender_account);
    println!("Transaction signature: {}", sig);

    // STEP 3: Create recipient token account with confidential transfer extension
    println!("\n=== STEP 3: Create confidential token account (recipient) ===");
    let recipient_owner = Keypair::new();
    println!("Recipient owner: {}", recipient_owner.pubkey());
    
    // Fund recipient owner with SOL
    let fund_sig = confidential_ops::fund_account(
        rpc_client.clone(),
        &payer,
        &recipient_owner.pubkey(),
        100_000_000, // 0.1 SOL
    ).await?;
    println!("Funded recipient with SOL, signature: {}", fund_sig);
    
    let (recipient_account, recipient_elgamal_keypair, recipient_ae_key, sig) = 
        confidential_ops::create_confidential_token_account(
            rpc_client.clone(),
            &recipient_owner,
            &mint_pubkey,
            &recipient_owner,
        ).await?;
    
    println!("Recipient account created: {}", recipient_account);
    println!("Transaction signature: {}", sig);
    
    // STEP 4: Mint tokens to sender (public balance)
    println!("\n=== STEP 4: Mint tokens to sender's public balance ===");
    let amount = 100 * 10u64.pow(decimals as u32); // 100 tokens
    let mint_sig = confidential_ops::mint_florin(
        rpc_client.clone(),
        &mint_pubkey,
        &sender_account,
        &payer,
        amount,
        decimals,
    ).await?;
    
    println!("Minted {} tokens to sender", amount as f64 / 10.0_f64.powi(decimals as i32));
    println!("Transaction signature: {}", mint_sig);
    
    // Check sender's public balance
    let sender_balance = confidential_ops::get_public_balance(
        rpc_client.clone(),
        &sender_account,
    ).await?;
    
    println!("Sender public balance: {}", sender_balance);
    
    // STEP 5: Deposit tokens to confidential pending balance
    println!("\n=== STEP 5: Deposit tokens to confidential pending balance ===");
    let deposit_sig = confidential_ops::deposit_ct(
        rpc_client.clone(),
        &sender_account,
        &payer,
        amount,
        decimals,
        &sender_elgamal_keypair,
    ).await?;
    
    println!("Deposited tokens to confidential pending balance");
    println!("Transaction signature: {}", deposit_sig);
    
    // STEP 6: Apply pending balance to make tokens available
    println!("\n=== STEP 6: Apply pending balance to make tokens available ===");
    let apply_sig = confidential_ops::apply_pending(
        rpc_client.clone(),
        &sender_account,
        &payer,
        &sender_elgamal_keypair,
        &sender_ae_key,
    ).await?;
    
    println!("Applied pending balance to make tokens available");
    println!("Transaction signature: {}", apply_sig);
    
    // STEP 7: Transfer tokens confidentially
    println!("\n=== STEP 7: Transfer tokens confidentially ===");
    let transfer_amount = 50 * 10u64.pow(decimals as u32); // 50 tokens
    let transfer_sig = confidential_ops::transfer_ct(
        rpc_client.clone(),
        &sender_account,
        &recipient_account,
        &payer,
        &sender_elgamal_keypair,
        &sender_ae_key,
        &recipient_elgamal_keypair.public(),
        transfer_amount,
        decimals,
    ).await?;
    
    println!("Transferred {} tokens confidentially", 
        transfer_amount as f64 / 10.0_f64.powi(decimals as i32));
    println!("Transaction signature: {}", transfer_sig);
    
    // Apply pending balance on recipient account
    println!("\n=== Apply pending balance for recipient ===");
    let apply_recipient_sig = confidential_ops::apply_pending(
        rpc_client.clone(),
        &recipient_account,
        &recipient_owner,
        &recipient_elgamal_keypair,
        &recipient_ae_key,
    ).await?;
    
    println!("Applied pending balance for recipient");
    println!("Transaction signature: {}", apply_recipient_sig);
    
    // STEP 8: Withdraw tokens from confidential to public balance
    println!("\n=== STEP 8: Withdraw tokens from confidential to public balance ===");
    let withdraw_amount = 25 * 10u64.pow(decimals as u32); // 25 tokens
    let withdraw_sig = confidential_ops::withdraw_ct(
        rpc_client.clone(),
        &recipient_account,
        &recipient_owner,
        &recipient_elgamal_keypair,
        &recipient_ae_key,
        withdraw_amount,
        decimals,
    ).await?;
    
    println!("Withdrew {} tokens from confidential to public balance", 
        withdraw_amount as f64 / 10.0_f64.powi(decimals as i32));
    println!("Transaction signature: {}", withdraw_sig);
    
    // STEP 9: Get balances
    println!("\n=== STEP 9: Get balances ===");
    
    // Get sender's balances
    let sender_public = confidential_ops::get_public_balance(
        rpc_client.clone(),
        &sender_account,
    ).await?;
    
    let (sender_pending, sender_available, _) = confidential_ops::get_ct_balance(
        rpc_client.clone(),
        &sender_account,
        &sender_elgamal_keypair,
        &sender_ae_key,
    ).await?;
    
    println!("Sender balances:");
    println!("  Public: {}", sender_public);
    println!("  Pending: {}", sender_pending);
    println!("  Available: {}", sender_available);
    
    // Get recipient's balances
    let recipient_public = confidential_ops::get_public_balance(
        rpc_client.clone(),
        &recipient_account,
    ).await?;
    
    let (recipient_pending, recipient_available, _) = confidential_ops::get_ct_balance(
        rpc_client.clone(),
        &recipient_account,
        &recipient_elgamal_keypair,
        &recipient_ae_key,
    ).await?;
    
    println!("Recipient balances:");
    println!("  Public: {}", recipient_public);
    println!("  Pending: {}", recipient_pending);
    println!("  Available: {}", recipient_available);
    
    println!("\n=== All steps completed successfully! ===");
    
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