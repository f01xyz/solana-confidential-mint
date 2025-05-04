use anyhow::{anyhow, Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer, read_keypair_file},
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
use florin_core::confidential_ops;
use florin_core::{
    proof_import::{import_and_verify_proof, ProofType},
    proof_verification::{verify_proof, VerificationConfig},
};
use std::path::Path;

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
    
    let rpc_client = Arc::new(RpcClient::new_with_timeout(
        String::from(rpc_url),
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

    // Example of importing and using a verified proof
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "create-mint" => {
                // Create a mint with confidential transfer extension
                println!("\nCreating a mint with confidential transfer extension...");
                let mint_pubkey = create_confidential_mint(
                    rpc_client.clone(),
                    &payer,
                    &mint,
                    decimals,
                ).await?;
                
                println!("Mint created successfully with address: {}", mint_pubkey);
            },
            "create-account" => {
                // Parse arguments
                let keypair_path = if let Some(arg) = std::env::args().nth(2) {
                    if arg == "--keypair" {
                        std::env::args().nth(3)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let owner_keypair_path = if let Some(arg) = std::env::args().nth(4) {
                    if arg == "--owner-keypair" {
                        std::env::args().nth(5)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let mint_address = if let Some(arg) = std::env::args().nth(6) {
                    if arg == "--mint" {
                        std::env::args().nth(7)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Check for output-address-only flag
                let output_address_only = std::env::args().any(|arg| arg == "--output-address-only");
                
                // Load the payer keypair
                let payer = if let Some(path) = keypair_path {
                    read_keypair_file(path).context("Failed to read keypair file")?
                } else {
                    // Default to default keypair if no keypair specified
                    payer.insecure_clone()
                };
                
                // Load the owner keypair
                let owner = if let Some(path) = owner_keypair_path {
                    read_keypair_file(path).context("Failed to read owner keypair file")?
                } else {
                    // Default to payer if no owner keypair specified
                    payer.insecure_clone()
                };
                
                // Parse mint address
                let mint = if let Some(addr) = mint_address {
                    addr.parse::<Pubkey>().context("Failed to parse mint address")?
                } else {
                    return Err(anyhow!("Mint address required. Usage: create-account --keypair <PATH> --owner-keypair <PATH> --mint <ADDRESS>"));
                };
                
                // Create the token account
                let (token_account, _, _, _) = confidential_ops::create_confidential_token_account(
                    rpc_client.clone(),
                    &payer,
                    &mint,
                    &owner,
                ).await?;
                
                if output_address_only {
                    // Output only the token account address
                    println!("{}", token_account);
                } else {
                    println!("Token account created successfully!");
                    println!("Token Account: {}", token_account);
                }
            },
            "import-transfer-proof" => {
                if let Some(proof_path) = std::env::args().nth(2) {
                    let source_account = if let Some(arg) = std::env::args().nth(3) {
                        arg.parse::<Pubkey>()?
                    } else {
                        return Err(anyhow!("Source account required"));
                    };
                    
                    let destination_account = if let Some(arg) = std::env::args().nth(4) {
                        arg.parse::<Pubkey>()?
                    } else {
                        return Err(anyhow!("Destination account required"));
                    };
                    
                    let amount = if let Some(arg) = std::env::args().nth(5) {
                        arg.parse::<u64>()?
                    } else {
                        return Err(anyhow!("Amount required"));
                    };
                    
                    println!("Importing and verifying transfer proof from {}...", proof_path);
                    let proof = import_and_verify_proof(Path::new(&proof_path))?;
                    
                    if proof.proof_type != ProofType::Transfer {
                        return Err(anyhow!("Expected a transfer proof"));
                    }
                    
                    println!("Proof verified successfully!");
                    
                    // Use the proof for a confidential transfer
                    let signature = confidential_ops::transfer_ct_with_proof(
                        rpc_client.clone(),
                        &source_account,
                        &destination_account,
                        &payer,
                        amount,
                        decimals,
                        &proof,
                    ).await?;
                    
                    println!("Confidential transfer completed with signature: {}", signature);
                } else {
                    return Err(anyhow!("Proof file path required"));
                }
            },
            "import-withdraw-proof" => {
                if let Some(proof_path) = std::env::args().nth(2) {
                    let token_account = if let Some(arg) = std::env::args().nth(3) {
                        arg.parse::<Pubkey>()?
                    } else {
                        return Err(anyhow!("Token account required"));
                    };
                    
                    let amount = if let Some(arg) = std::env::args().nth(4) {
                        arg.parse::<u64>()?
                    } else {
                        return Err(anyhow!("Amount required"));
                    };
                    
                    println!("Importing and verifying withdraw proof from {}...", proof_path);
                    let proof = import_and_verify_proof(Path::new(&proof_path))?;
                    
                    if proof.proof_type != ProofType::Withdraw {
                        return Err(anyhow!("Expected a withdraw proof"));
                    }
                    
                    println!("Proof verified successfully!");
                    
                    // Use the proof for a confidential withdrawal
                    let signature = confidential_ops::withdraw_ct_with_proof(
                        rpc_client.clone(),
                        &token_account,
                        &payer,
                        amount,
                        decimals,
                        &proof,
                    ).await?;
                    
                    println!("Confidential withdrawal completed with signature: {}", signature);
                } else {
                    return Err(anyhow!("Proof file path required"));
                }
            },
            "confidential-transfer" => {
                // Parse arguments
                let keypair_path = if let Some(arg) = std::env::args().nth(2) {
                    if arg == "--keypair" {
                        std::env::args().nth(3)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let proof_path = if let Some(arg) = std::env::args().nth(4) {
                    if arg == "--proof-path" {
                        std::env::args().nth(5)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Load the owner keypair
                let owner = if let Some(path) = keypair_path {
                    read_keypair_file(path).context("Failed to read keypair file")?
                } else {
                    // Default to payer if no keypair specified
                    payer.insecure_clone()
                };
                
                // Ensure we have a proof path
                let proof_file = proof_path.ok_or_else(|| 
                    anyhow!("Proof path required. Usage: confidential-transfer --keypair <PATH> --proof-path <PATH>")
                )?;
                
                println!("Performing confidential transfer with proof from {}...", proof_file);
                
                // Call the confidential_transfer function
                let signature = confidential_ops::confidential_transfer(
                    rpc_client.clone(),
                    &owner,
                    Path::new(&proof_file),
                ).await?;
                
                println!("Confidential transfer completed successfully!");
                println!("Transaction signature: {}", signature);
            },
            "mint" => {
                // Parse arguments
                let keypair_path = if let Some(arg) = std::env::args().nth(2) {
                    if arg == "--keypair" {
                        std::env::args().nth(3)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let mint_address = if let Some(arg) = std::env::args().nth(4) {
                    if arg == "--mint" {
                        std::env::args().nth(5)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let destination = if let Some(arg) = std::env::args().nth(6) {
                    if arg == "--destination" {
                        std::env::args().nth(7)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let amount = if let Some(arg) = std::env::args().nth(8) {
                    if arg == "--amount" {
                        if let Some(amt) = std::env::args().nth(9) {
                            amt.parse::<u64>().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Load the payer keypair
                let payer = if let Some(path) = keypair_path {
                    read_keypair_file(path).context("Failed to read keypair file")?
                } else {
                    // Default to default keypair if no keypair specified
                    payer.insecure_clone()
                };
                
                // Parse mint address
                let mint = if let Some(addr) = mint_address {
                    addr.parse::<Pubkey>().context("Failed to parse mint address")?
                } else {
                    return Err(anyhow!("Mint address required. Usage: mint --keypair <PATH> --mint <ADDRESS> --destination <ADDRESS> --amount <AMOUNT>"));
                };
                
                // Parse destination address
                let destination_address = if let Some(addr) = destination {
                    addr.parse::<Pubkey>().context("Failed to parse destination address")?
                } else {
                    return Err(anyhow!("Destination address required. Usage: mint --keypair <PATH> --mint <ADDRESS> --destination <ADDRESS> --amount <AMOUNT>"));
                };
                
                // Parse amount
                let mint_amount = if let Some(amt) = amount {
                    amt
                } else {
                    return Err(anyhow!("Amount required. Usage: mint --keypair <PATH> --mint <ADDRESS> --destination <ADDRESS> --amount <AMOUNT>"));
                };
                
                // Mint tokens
                let signature = confidential_ops::mint_florin(
                    rpc_client.clone(),
                    &mint,
                    &destination_address,
                    &payer,
                    mint_amount,
                    decimals,
                ).await?;
                
                println!("Tokens minted successfully!");
                println!("Transaction signature: {}", signature);
            },
            "deposit" => {
                // Parse arguments
                let keypair_path = if let Some(arg) = std::env::args().nth(2) {
                    if arg == "--keypair" {
                        std::env::args().nth(3)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let account = if let Some(arg) = std::env::args().nth(4) {
                    if arg == "--account" {
                        std::env::args().nth(5)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let amount = if let Some(arg) = std::env::args().nth(6) {
                    if arg == "--amount" {
                        if let Some(amt) = std::env::args().nth(7) {
                            amt.parse::<u64>().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Load the owner keypair
                let owner = if let Some(path) = keypair_path {
                    read_keypair_file(path).context("Failed to read keypair file")?
                } else {
                    // Default to default keypair if no keypair specified
                    payer.insecure_clone()
                };
                
                // Parse account address
                let token_account = if let Some(addr) = account {
                    addr.parse::<Pubkey>().context("Failed to parse account address")?
                } else {
                    return Err(anyhow!("Account address required. Usage: deposit --keypair <PATH> --account <ADDRESS> --amount <AMOUNT>"));
                };
                
                // Parse amount
                let deposit_amount = if let Some(amt) = amount {
                    amt
                } else {
                    return Err(anyhow!("Amount required. Usage: deposit --keypair <PATH> --account <ADDRESS> --amount <AMOUNT>"));
                };
                
                // Create ElGamal keypair (normally this would be loaded from a file)
                let elgamal_keypair = ElGamalKeypair::new_rand();
                
                // Deposit tokens
                let signature = confidential_ops::deposit_ct(
                    rpc_client.clone(),
                    &token_account,
                    &owner,
                    deposit_amount,
                    decimals,
                    &elgamal_keypair,
                ).await?;
                
                println!("Tokens deposited to confidential pending balance!");
                println!("Transaction signature: {}", signature);
                
                // Apply pending balance
                let ae_key = AeKey::new_rand();
                let apply_signature = confidential_ops::apply_pending(
                    rpc_client.clone(),
                    &token_account,
                    &owner,
                    &elgamal_keypair,
                    &ae_key,
                ).await?;
                
                println!("Pending balance applied to available balance!");
                println!("Transaction signature: {}", apply_signature);
            },
            "get-balance" => {
                // Parse account argument
                let account = if let Some(arg) = std::env::args().nth(2) {
                    if arg == "--account" {
                        std::env::args().nth(3)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Parse account address
                let token_account = if let Some(addr) = account {
                    addr.parse::<Pubkey>().context("Failed to parse account address")?
                } else {
                    return Err(anyhow!("Account address required. Usage: get-balance --account <ADDRESS>"));
                };
                
                // Get public balance
                let public_balance = confidential_ops::get_public_balance(
                    rpc_client.clone(),
                    &token_account,
                ).await?;
                
                println!("Public balance: {}", public_balance);
                
                // We can't get confidential balance without ElGamal and AES keys
                println!("Note: Confidential balance requires encryption keys and is not shown.");
            },
            _ => {
                // Show available commands
                println!("Florin Core - Confidential Transfer CLI");
                println!("\nAvailable commands:");
                println!("  create-mint --keypair <PATH> --mint-keypair <PATH> --decimals <NUMBER>");
                println!("  create-account --keypair <PATH> --owner-keypair <PATH> --mint <ADDRESS> [--output-address-only]");
                println!("  mint --keypair <PATH> --mint <ADDRESS> --destination <ADDRESS> --amount <NUMBER>");
                println!("  deposit --keypair <PATH> --account <ADDRESS> --amount <NUMBER>");
                println!("  get-balance --account <ADDRESS>");
                println!("  confidential-transfer --keypair <PATH> --proof-path <PATH>");
                println!("  import-transfer-proof <PROOF_PATH> <SOURCE_ACCOUNT> <DESTINATION_ACCOUNT> <AMOUNT>");
                println!("  import-withdraw-proof <PROOF_PATH> <TOKEN_ACCOUNT> <AMOUNT>");
                println!("\nFor more detailed help, run with --help");
                
                return Err(anyhow!("Unknown command: {}", arg));
            }
        }
        
        return Ok(());
    }
    
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

    // Create a dummy proof for demonstration purposes
    let transfer_proof = florin_core::proof_import::ImportableProof {
        proof_type: florin_core::proof_import::ProofType::Transfer,
        data: vec![1; 100], // 100 bytes of dummy data
        metadata: florin_core::proof_import::ProofMetadata {
            source_pubkey: Some(sender_account.to_string()),
            destination_pubkey: Some(recipient_account.to_string()),
            amount: Some(amount),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    };

    // Transfer some tokens confidentially
    let transfer_amount = 50 * 10u64.pow(decimals as u32); // 50 tokens
    confidential_ops::transfer_ct_with_proof(
        rpc_client.clone(),
        &sender_account,
        &recipient_account,
        &payer,
        transfer_amount,
        decimals,
        &transfer_proof,
    ).await?;
    
    // Apply the pending balance on the recipient account
    confidential_ops::apply_pending(
        rpc_client.clone(),
        &recipient_account,
        &recipient_owner,
        &recipient_elgamal_keypair,
        &recipient_ae_key,
    ).await?;

    // Create a dummy proof for demonstration purposes
    let withdraw_proof = florin_core::proof_import::ImportableProof {
        proof_type: florin_core::proof_import::ProofType::Withdraw,
        data: vec![1; 100], // 100 bytes of dummy data
        metadata: florin_core::proof_import::ProofMetadata {
            source_pubkey: Some(recipient_account.to_string()),
            destination_pubkey: None,
            amount: Some(transfer_amount),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    };

    // Withdraw half of the tokens from recipient's confidential balance to public
    let withdraw_amount = 25 * 10u64.pow(decimals as u32); // 25 tokens
    confidential_ops::withdraw_ct_with_proof(
        rpc_client.clone(),
        &recipient_account,
        &recipient_owner,
        withdraw_amount,
        decimals,
        &withdraw_proof,
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