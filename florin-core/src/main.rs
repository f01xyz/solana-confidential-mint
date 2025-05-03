use anyhow::{anyhow, Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token_2022::{
    instruction::{
        initialize_mint as initialize_token_mint,
        initialize_account as initialize_token_account,
        mint_to,
    },
};
use std::time::Duration;
use std::sync::Arc;
use spl_token_client::{
    client::{ProgramRpcClient, ProgramRpcClientSendTransaction},
    token::{ExtensionInitializationParams, Token},
};

// Our library with confidential transfer operations
use lib::confidential_ops;

// Size of a mint account
const MINT_SIZE: usize = 82;

// Size of a token account
const TOKEN_ACCOUNT_SIZE: usize = 165;

// Amount to mint (in the smallest denomination)
const MINT_AMOUNT: u64 = 1_000_000_000; // 1 token with 9 decimals

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

    // Wait for validator to be ready
    println!("Waiting for validator to be ready...");
    let mut attempts = 0;
    while attempts < 5 {
        match rpc_client.get_health().await {
            Ok(_) => {
                println!("Validator is ready!");
                break;
            },
            Err(_) => {
                println!("Waiting for validator to be ready... (attempt {})", attempts + 1);
                tokio::time::sleep(Duration::from_secs(2)).await;
                attempts += 1;
            }
        }
        if attempts == 5 {
            println!("Validator not responding. Make sure it's running.");
            println!("Run: solana-test-validator --clone-upgradeable-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb --url https://api.mainnet-beta.solana.com -r");
            return Ok(());
        }
    }

    // Load keypair from default location
    let payer = load_keypair()?;
    println!("Using payer: {}", payer.pubkey());

    // Generate a new keypair for the mint
    let mint = Keypair::new();
    println!("Mint keypair generated: {}", mint.pubkey());

    // Number of decimals for the mint
    let decimals = 2;

    // Create a mint with confidential transfer extension
    println!("\nCreating a mint with confidential transfer extension...");
    let mint_pubkey = create_confidential_mint(
        rpc_client.clone(),
        &payer,
        &mint,
        decimals,
    ).await?;
    
    println!("Mint created successfully with address: {}", mint_pubkey);
    
    Ok(())
}

async fn basic_token_example() -> Result<()> {
    // Create connection to local test validator
    let rpc_url = "http://localhost:8899";
    println!("Connecting to Solana validator at {}", rpc_url);
    println!("Note: You must have a local validator running with token-2022 program cloned.");
    
    let rpc_client = RpcClient::new_with_commitment(
        String::from(rpc_url),
        CommitmentConfig::confirmed(),
    );

    // Load the default Solana CLI keypair to use as the fee payer
    // This will be the wallet paying for the transaction fees
    let payer = load_keypair()?;
    println!("Using payer: {}", payer.pubkey());

    // Generate a new keypair to use as the address of the token mint
    let mint = Keypair::new();
    println!("Mint keypair generated: {}", mint.pubkey());

    // Number of decimals for the mint
    let decimals = 9;

    // Calculate necessary space for a regular mint account
    let space = MINT_SIZE;

    // Calculate minimum rent exemption balance
    let rent = rpc_client.get_minimum_balance_for_rent_exemption(space).await?;

    // Create the mint account
    let create_account_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        space as u64,
        &spl_token_2022::id(),
    );

    // Initialize the mint with standard token properties
    let initialize_mint_instruction = initialize_token_mint(
        &spl_token_2022::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        Some(&payer.pubkey()),  // Freeze authority
        decimals,
    )?;

    // Combine all instructions into a single transaction
    let instructions = vec![
        create_account_instruction,
        initialize_mint_instruction,
    ];

    // Create and sign the transaction
    let blockhash = rpc_client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[&payer, &mint],
        blockhash,
    );

    // Send and confirm the transaction
    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await?;

    println!("Transaction successful!");
    println!("Mint Address: {}", mint.pubkey());
    println!("Transaction Signature: {}", signature);

    // Verify the mint was created
    verify_mint(&rpc_client, &mint.pubkey()).await?;

    // Create a token account and mint tokens
    create_token_account_and_mint(&rpc_client, &payer, &mint.pubkey(), MINT_AMOUNT, decimals).await?;

    Ok(())
}

async fn confidential_transfer_example() -> Result<()> {
    // Create connection to local test validator with confirmed commitment level
    let rpc_url = "http://localhost:8899";
    println!("Connecting to Solana validator at {}", rpc_url);
    println!("Note: You must have a local validator running with token-2022 program cloned.");
    
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        String::from(rpc_url),
        CommitmentConfig::confirmed(),
    ));

    // Load the default Solana CLI keypair to use as the fee payer
    let payer = load_keypair()?;
    println!("Using payer: {}", payer.pubkey());

    // Generate a new keypair for the mint
    let mint = Keypair::new();
    println!("Mint keypair generated: {}", mint.pubkey());

    // Number of decimals for the mint
    let decimals = 2;

    // Create a mint with confidential transfer extension
    println!("\nCreating mint with confidential transfer extension...");
    let (mint_pubkey, _) = confidential_ops::create_confidential_mint(
        rpc_client.clone(),
        &payer,
        &mint,
        decimals,
    ).await?;

    // Create the sender's token account with confidential transfer extension
    println!("\nCreating sender token account with confidential transfer extension...");
    let (sender_account, sender_elgamal_keypair, sender_ae_key, _) = 
        confidential_ops::create_confidential_token_account(
            rpc_client.clone(),
            &payer,
            &mint_pubkey,
            &payer,
        ).await?;
    
    println!("Sender account created: {}", sender_account);

    // Create the recipient's token account with a different owner
    println!("\nCreating recipient token account with confidential transfer extension...");
    let recipient_owner = Keypair::new();
    println!("Recipient owner: {}", recipient_owner.pubkey());

    // Fund the recipient with some SOL for transaction fees
    confidential_ops::fund_account(
        rpc_client.clone(),
        &payer,
        &recipient_owner.pubkey(),
        100_000_000, // 0.1 SOL in lamports
    ).await?;

    let (recipient_account, recipient_elgamal_keypair, recipient_ae_key, _) = 
        confidential_ops::create_confidential_token_account(
            rpc_client.clone(),
            &recipient_owner, // Funded owner pays for account creation
            &mint_pubkey,
            &recipient_owner,
        ).await?;
    
    println!("Recipient account created: {}", recipient_account);

    // Mint tokens to the sender's account (public balance)
    let amount = 100 * 10u64.pow(decimals as u32); // 100 tokens
    confidential_ops::mint_florin(
        rpc_client.clone(),
        &mint_pubkey,
        &sender_account,
        &payer,
        amount,
        decimals,
    ).await?;
    
    // Get the current public balance
    let public_balance = confidential_ops::get_public_balance(
        rpc_client.clone(),
        &sender_account,
    ).await?;
    
    println!("\nPublic balance after minting: {}", public_balance);
    
    // Deposit tokens to confidential pending balance
    confidential_ops::deposit_ct(
        rpc_client.clone(),
        &sender_account,
        &payer,
        amount,
        decimals,
        &sender_elgamal_keypair,
    ).await?;

    // Apply pending balance to make tokens available
    confidential_ops::apply_pending(
        rpc_client.clone(),
        &sender_account,
        &payer,
        &sender_elgamal_keypair,
        &sender_ae_key,
    ).await?;

    // Transfer some tokens confidentially
    let transfer_amount = 50 * 10u64.pow(decimals as u32); // 50 tokens
    confidential_ops::transfer_ct(
        rpc_client.clone(),
        &sender_account,
        &recipient_account,
        &payer,
        &sender_elgamal_keypair,
        &sender_ae_key,
        &recipient_elgamal_keypair.public,
        transfer_amount,
        decimals,
    ).await?;

    // Apply the pending balance on the recipient account
    confidential_ops::apply_pending(
        rpc_client.clone(),
        &recipient_account,
        &recipient_owner,
        &recipient_elgamal_keypair,
        &recipient_ae_key,
    ).await?;

    // Withdraw half of the tokens from recipient's confidential balance to public
    let withdraw_amount = 25 * 10u64.pow(decimals as u32); // 25 tokens
    confidential_ops::withdraw_ct(
        rpc_client.clone(),
        &recipient_account,
        &recipient_owner,
        &recipient_elgamal_keypair,
        &recipient_ae_key,
        withdraw_amount,
        decimals,
    ).await?;

    // Get final balances
    let sender_public_balance = confidential_ops::get_public_balance(
        rpc_client.clone(),
        &sender_account,
    ).await?;
    
    let recipient_public_balance = confidential_ops::get_public_balance(
        rpc_client.clone(),
        &recipient_account,
    ).await?;
    
    // Get confidential balances
    let (sender_pending, sender_available, _) = confidential_ops::get_ct_balance(
        rpc_client.clone(),
        &sender_account,
        &sender_elgamal_keypair,
        &sender_ae_key,
    ).await?;
    
    let (recipient_pending, recipient_available, _) = confidential_ops::get_ct_balance(
        rpc_client.clone(),
        &recipient_account,
        &recipient_elgamal_keypair,
        &recipient_ae_key,
    ).await?;

    println!("\nConfidential transfer flow demonstration completed!");
    println!("Sender Account: {}", sender_account);
    println!("Recipient Account: {}", recipient_account);
    
    println!("\nFinal balances - Sender:");
    println!("Public balance: {}", sender_public_balance);
    println!("Pending balance: {}", sender_pending);
    println!("Available balance: {}", sender_available);
    
    println!("\nFinal balances - Recipient:");
    println!("Public balance: {}", recipient_public_balance);
    println!("Pending balance: {}", recipient_pending);
    println!("Available balance: {}", recipient_available);

    Ok(())
}

/// Fund an account with SOL for transaction fees
async fn fund_account(
    client: Arc<RpcClient>,
    payer: &Keypair,
    recipient: &Pubkey,
    lamports: u64,
) -> Result<()> {
    println!("\nFunding account {} with {} SOL", recipient, lamports as f64 / 1_000_000_000.0);
    
    let transfer_instruction = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        recipient,
        lamports,
    );
    
    let blockhash = client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );
    
    let signature = client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await?;
    
    println!("Account funded: {}", signature);
    Ok(())
}

async fn create_token_account_and_mint(
    client: &RpcClient,
    payer: &Keypair,
    mint_address: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    println!("\nCreating token account and minting tokens...");

    // Generate a new keypair for the token account
    let token_account = Keypair::new();
    println!("Token account keypair generated: {}", token_account.pubkey());

    // Calculate minimum rent exemption balance for token account
    let rent = client.get_minimum_balance_for_rent_exemption(TOKEN_ACCOUNT_SIZE).await?;

    // Create the token account
    let create_account_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        rent,
        TOKEN_ACCOUNT_SIZE as u64,
        &spl_token_2022::id(),
    );

    // Initialize the token account
    let initialize_account_instruction = initialize_token_account(
        &spl_token_2022::id(),
        &token_account.pubkey(),
        mint_address,
        &payer.pubkey(),
    )?;

    // Mint tokens to the token account
    let mint_to_instruction = mint_to(
        &spl_token_2022::id(),
        mint_address,
        &token_account.pubkey(),
        &payer.pubkey(),
        &[],
        amount,
    )?;

    // Combine all instructions into a single transaction
    let instructions = vec![
        create_account_instruction,
        initialize_account_instruction,
        mint_to_instruction,
    ];

    // Create and sign the transaction
    let blockhash = client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[payer, &token_account],
        blockhash,
    );

    // Send and confirm the transaction
    let signature = client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await?;

    println!("Tokens minted successfully!");
    println!("Token Account: {}", token_account.pubkey());
    println!("Transaction Signature: {}", signature);

    // Fetch and display the token balance
    let account_data = client.get_account(&token_account.pubkey()).await?;
    println!("Token account data size: {} bytes", account_data.data.len());
    
    // To parse the token account data, we'd typically use TokenAccount::unpack
    // but for simplicity, we'll just display the amount we know we minted
    let token_amount = amount as f64 / 10f64.powi(decimals as i32);
    println!("Token balance: {} tokens", token_amount);
    println!("✅ Tokens minted successfully!");

    Ok(())
}

async fn verify_mint(client: &RpcClient, mint_address: &Pubkey) -> Result<()> {
    println!("Verifying mint...");
    
    // Allow some time for the transaction to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Get the account data
    let account = client.get_account(mint_address).await?;
    
    // Check that this is a token account
    if account.owner != spl_token_2022::id() {
        return Err(anyhow!("Account is not owned by the Token-2022 program"));
    }
    
    println!("✅ Account is owned by the Token-2022 program");
    println!("Account data size: {} bytes", account.data.len());
    println!("✅ Mint created successfully!");
    
    Ok(())
}

// Load the keypair from the default Solana CLI keypair path (~/.config/solana/id.json)
// This enables using the same wallet as the Solana CLI tools
fn load_keypair() -> Result<Keypair> {
    // Get the default keypair path
    let keypair_path = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".config/solana/id.json");

    // Read the keypair file directly into bytes using serde_json
    // The keypair file is a JSON array of bytes
    let file = std::fs::File::open(&keypair_path)?;
    let keypair_bytes: Vec<u8> = serde_json::from_reader(file)?;

    // Create keypair from the loaded bytes
    // This converts the byte array into a keypair
    let keypair = Keypair::from_bytes(&keypair_bytes)?;

    Ok(keypair)
}

/// Create a mint with confidential transfer extension
async fn create_confidential_mint(
    rpc_client: Arc<RpcClient>,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
) -> Result<Pubkey> {
    // Set up program client for Token client
    let program_client = ProgramRpcClient::new(
        rpc_client.clone(),
        ProgramRpcClientSendTransaction
    );
    
    // Create a token client for the Token-2022 program
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(), // Use the Token-2022 program
        &mint.pubkey(),        // Address of the new token mint
        Some(decimals),        // Number of decimal places
        Arc::new(payer.insecure_clone()), // Fee payer for transactions
    );
    
    // Create extension initialization parameters for the mint
    let extension_initialization_params = vec![
        ExtensionInitializationParams::ConfidentialTransferMint {
            authority: Some(payer.pubkey()), // Authority that can modify confidential transfer settings
            auto_approve_new_accounts: true, // Automatically approve new confidential accounts
            auditor_elgamal_pubkey: None,    // None if no auditor
        }
    ];
    
    // Create and initialize the mint with the ConfidentialTransferMint extension
    let signature = token
        .create_mint(
            &payer.pubkey(),                 // Mint authority - can mint new tokens
            Some(&payer.pubkey()),           // Freeze authority - can freeze token accounts
            extension_initialization_params,  // Add the ConfidentialTransferMint extension
            &[mint],                         // Mint keypair needed as signer
        )
        .await?;
    
    println!("Transaction signature: {}", signature);
    
    // Verify mint account
    println!("Verifying mint account...");
    let account = rpc_client.get_account(&mint.pubkey()).await?;
    println!("Mint account exists with {} bytes of data", account.data.len());
    
    Ok(mint.pubkey())
} 