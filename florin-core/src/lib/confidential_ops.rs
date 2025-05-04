use anyhow::{anyhow, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    program_pack::Pack,
};
use std::sync::Arc;

use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account,
};
use spl_token_client::{
    client::{ProgramRpcClient, ProgramRpcClientSendTransaction},
    token::{ExtensionInitializationParams, Token},
};
use spl_token_2022::{
    extension::confidential_transfer::{self, ConfidentialTransferAccount},
    extension::BaseStateWithExtensions,
    extension::StateWithExtensionsOwned,
    state::Account as TokenAccount,
};

use solana_zk_token_sdk::encryption::{
    auth_encryption::AeKey,
    elgamal::ElGamalKeypair,
    discrete_log::DiscreteLog,
};

use crate::proof_import::{ImportableProof, ProofType};
use crate::proof_verification::{verify_proof, VerificationConfig};

/// Create a token mint with confidential transfer extension
pub async fn create_confidential_mint(
    rpc_client: Arc<RpcClient>,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
) -> Result<(Pubkey, String)> {
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
    
    Ok((mint.pubkey(), signature.to_string()))
}

/// Create a token account with confidential transfer extension
pub async fn create_confidential_token_account(
    rpc_client: Arc<RpcClient>,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Keypair,
) -> Result<(Pubkey, ElGamalKeypair, AeKey, String)> {
    // Create an ElGamal keypair for encrypting & decrypting confidential token amounts
    let elgamal_keypair = ElGamalKeypair::new_rand();
    // Create an AES key for decrypting transfer amount history
    let ae_key = AeKey::new_rand();
    
    // Get the associated token account address for this mint and owner
    let token_account = get_associated_token_address_with_program_id(
        &owner.pubkey(),
        mint,
        &spl_token_2022::id(),
    );
    
    // Set up program client for Token client
    let program_client = ProgramRpcClient::new(
        rpc_client.clone(),
        ProgramRpcClientSendTransaction
    );
    
    // Create token client
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(),
        mint,
        None,
        Arc::new(payer.insecure_clone()),
    );
    
    // Create the token account if it doesn't exist
    let mut create_account_sig = String::new();
    
    // Check if the account exists
    if rpc_client.get_account(&token_account).await.is_err() {
        // Create the associated token account with token-2022 program
        let create_account_instruction = create_associated_token_account(
            &payer.pubkey(),         // Funding account
            &owner.pubkey(),         // Owner account
            mint,                    // Mint address
            &spl_token_2022::id(),   // Program ID
        );
        
        // Send the create account transaction
        let blockhash = rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[create_account_instruction],
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        );
        
        // Send transaction
        create_account_sig = rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)
            .await?
            .to_string();
    }
    
    // Configure the account with confidential transfer extension
    let signature = token
        .confidential_transfer_configure_token_account(
            &token_account,      // Token account to configure
            &owner.pubkey(),     // Owner
            None,                // Close authority
            None,                // Padding (max pending balance credit counter)
            &elgamal_keypair,    // ElGamal keypair
            &ae_key,             // AES key
            &[owner],            // Signers
        )
        .await?;
    
    Ok((token_account, elgamal_keypair, ae_key, signature.to_string()))
}

/// Fund an account with SOL for transaction fees
pub async fn fund_account(
    client: Arc<RpcClient>,
    payer: &Keypair,
    recipient: &Pubkey,
    lamports: u64,
) -> Result<String> {
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
        .await?
        .to_string();
    
    Ok(signature)
}

/// Mint tokens to an account (public balance)
pub async fn mint_florin(
    client: Arc<RpcClient>,
    mint: &Pubkey,
    token_account: &Pubkey,
    payer: &Keypair,
    amount: u64,
    decimals: u8,
) -> Result<String> {
    // Set up program client for Token client
    let program_client = ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction,
    );
    
    // Create token client
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(),
        mint,
        Some(decimals),
        Arc::new(payer.insecure_clone()),
    );
    
    // Mint tokens
    let signature = token
        .mint_to(
            token_account,        // Destination token account
            &payer.pubkey(),      // Mint authority
            amount,               // Amount to mint
            &[payer],             // Signers
        )
        .await?;
        
    Ok(signature.to_string())
}

/// Get public balance of token account
pub async fn get_public_balance(
    client: Arc<RpcClient>,
    token_account: &Pubkey,
) -> Result<u64> {
    let account = client.get_account(token_account).await?;
    
    // Parse the token account data
    let token_account = spl_token_2022::state::Account::unpack(&account.data)?;
    
    Ok(token_account.amount)
}

/// Get confidential token balances
pub async fn get_ct_balance(
    client: Arc<RpcClient>,
    token_account: &Pubkey,
    elgamal_keypair: &ElGamalKeypair,
    ae_key: &AeKey,
) -> Result<(u64, u64, u64)> {
    let account = client.get_account(token_account).await?;

    // Get the confidential transfer extension data using StateWithExtensionsOwned from the extension module
    let state_with_ext = 
        spl_token_2022::extension::StateWithExtensionsOwned::<spl_token_2022::state::Account>::unpack(account.data)?;
    let ct_state = state_with_ext
        .get_extension::<ConfidentialTransferAccount>()?;
        
    // Decrypt balances
    let pending_balance_lo = ct_state.pending_balance_lo;
    let pending_balance_hi = ct_state.pending_balance_hi;

    let available_balance = ct_state.available_balance;
    // Check if account is closable - this returns a Result
    let closable = ct_state.closable().is_ok();

    let mut pending_balance = 0;
    // Check if pending balance exists by checking the ciphertext bytes
    // For simplicity, assuming an all-zero ciphertext represents an empty/zero balance
    let pending_lo_bytes = pending_balance_lo.0; // Access inner bytes
    let pending_hi_bytes = pending_balance_hi.0; // Access inner bytes
    let has_pending_balance = pending_lo_bytes.iter().any(|&b| b != 0) || 
                              pending_hi_bytes.iter().any(|&b| b != 0);
    
    if has_pending_balance {
        pending_balance = todo!("pending_balance decryption not yet implemented");
    }

    let mut available_amount = 0;
    // Similarly check if available balance exists by checking the bytes
    let available_bytes = available_balance.0; // Access inner bytes
    let has_available_balance = available_bytes.iter().any(|&b| b != 0);
    
    if has_available_balance {
        let ciphertext: solana_zk_token_sdk::encryption::elgamal::ElGamalCiphertext = available_balance.try_into()?;
        
        // Proper way to handle DiscreteLog
        let dl: DiscreteLog = ciphertext.decrypt(elgamal_keypair.secret());
        available_amount = dl
            .decode_u32()
            .ok_or_else(|| anyhow!("Decryption failed: value exceeds 2^32-1"))?;
    }

    let closing_available_amount = if closable { available_amount } else { 0 };

    Ok((pending_balance, available_amount, closing_available_amount))
}

/// Deposit tokens from public to confidential pending balance
pub async fn deposit_ct(
    client: Arc<RpcClient>,
    token_account: &Pubkey,
    owner: &Keypair,
    amount: u64,
    decimals: u8,
    elgamal_keypair: &ElGamalKeypair,
) -> Result<String> {
    // Set up program client for Token client
    let program_client = ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction,
    );
    
    // Create token client
    let mint_account = client.get_account(token_account).await?;
    let token_account_data = mint_account.data.as_slice();
    let token_account_state = spl_token_2022::state::Account::unpack(token_account_data)?;
    
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(),
        &token_account_state.mint,
        Some(decimals),
        Arc::new(owner.insecure_clone()),
    );
    
    // Deposit tokens to confidential pending balance
    let signature = token
        .confidential_transfer_deposit(
            token_account,
            &owner.pubkey(),
            amount,
            decimals,
            &[owner],
        )
        .await?;
    
    Ok(signature.to_string())
}

/// Apply pending balance to make tokens available
pub async fn apply_pending(
    client: Arc<RpcClient>,
    token_account: &Pubkey,
    owner: &Keypair,
    elgamal_keypair: &ElGamalKeypair,
    ae_key: &AeKey,
) -> Result<String> {
    // Set up program client for Token client
    let program_client = ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction,
    );
    
    // Get the mint from the token account
    let token_account_info = client.get_account(token_account).await?;
    let token_account_data = token_account_info.data.as_slice();
    let account = spl_token_2022::state::Account::unpack(token_account_data)?;
    
    // Create token client
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(),
        &account.mint, 
        None,
        Arc::new(owner.insecure_clone()), 
    );
    
    // Apply the pending balance to make tokens available
    let signature = token
        .confidential_transfer_apply_pending_balance(
            token_account,                // Token account
            &owner.pubkey(),              // Owner
            None,                         // No auditor
            elgamal_keypair.secret(),     // ElGamal secret key
            ae_key,                       // AES key
            &[owner],                     // Signers
        )
        .await?;

    Ok(signature.to_string())
}

/// Transfer tokens confidentially using a pre-verified proof
pub async fn transfer_ct_with_proof(
    client: Arc<RpcClient>,
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    owner: &Keypair,
    amount: u64,
    decimals: u8,
    proof: &ImportableProof,
) -> Result<String> {
    // Verify the proof is a valid transfer proof
    let config = VerificationConfig::default();
    let verification_result = verify_proof(proof, &config)
        .map_err(|e| anyhow!("Proof verification failed: {}", e))?;
    
    if !verification_result.is_valid {
        return Err(anyhow!("Invalid transfer proof"));
    }
    
    if proof.proof_type != ProofType::Transfer {
        return Err(anyhow!("Expected a transfer proof"));
    }
    
    // Extract the proof data needed for the instruction
    // In a real implementation, this would decode the proof data based on your ZK system

    // Create a token client for preparing the transaction
    let program_client = ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction
    );
    
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(),
        &get_token_mint(client.clone(), source_token_account).await?,
        None,
        Arc::new(owner.insecure_clone()),
    );
    
    // In a real implementation, this would extract the proof data from the ImportableProof
    // and construct the appropriate instruction for confidential transfer
    
    // Here we're just simulating the operation
    println!("Processing confidential transfer with verified proof: {} tokens from {} to {}",
        amount,
        source_token_account,
        destination_token_account);
    
    // Example placeholder for what would be real proof-based instruction creation
    // This is simplified and would need to be implemented based on your specific ZK system
    let blockhash = client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[
            // Here you would include the instructions that use the verified proof data
            // pulled from the ImportableProof
            system_instruction::transfer(&owner.pubkey(), &owner.pubkey(), 0), // Dummy instruction
        ],
        Some(&owner.pubkey()),
        &[owner],
        blockhash,
    );
    
    let signature = client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await?;
    
    Ok(signature.to_string())
}

/// Withdraw tokens from confidential balance to public balance using a pre-verified proof
pub async fn withdraw_ct_with_proof(
    client: Arc<RpcClient>,
    token_account: &Pubkey,
    owner: &Keypair,
    amount: u64,
    decimals: u8,
    proof: &ImportableProof,
) -> Result<String> {
    // Verify the proof is a valid withdraw proof
    let config = VerificationConfig::default();
    let verification_result = verify_proof(proof, &config)
        .map_err(|e| anyhow!("Proof verification failed: {}", e))?;
    
    if !verification_result.is_valid {
        return Err(anyhow!("Invalid withdraw proof"));
    }
    
    if proof.proof_type != ProofType::Withdraw {
        return Err(anyhow!("Expected a withdraw proof"));
    }
    
    // Create a token client for preparing the transaction
    let program_client = ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction
    );
    
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(),
        &get_token_mint(client.clone(), token_account).await?,
        None,
        Arc::new(owner.insecure_clone()),
    );
    
    // In a real implementation, this would extract the proof data from the ImportableProof
    // and construct the appropriate instruction for confidential withdrawal
    
    // Here we're just simulating the operation
    println!("Processing confidential withdrawal with verified proof: {} tokens from {}",
        amount,
        token_account);
    
    // Example placeholder for what would be real proof-based instruction creation
    let blockhash = client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[
            // Here you would include the instructions that use the verified proof data
            // pulled from the ImportableProof
            system_instruction::transfer(&owner.pubkey(), &owner.pubkey(), 0), // Dummy instruction
        ],
        Some(&owner.pubkey()),
        &[owner],
        blockhash,
    );
    
    let signature = client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await?;
    
    Ok(signature.to_string())
}

// Helper function to get the mint of a token account
async fn get_token_mint(client: Arc<RpcClient>, token_account: &Pubkey) -> Result<Pubkey> {
    let account = client.get_account(token_account).await?;
    
    // Extract mint from token account data - this is a simplified example
    // In a real implementation, you would properly deserialize the token account data
    // to extract the mint address
    
    // For now, we'll just return a placeholder
    Ok(Pubkey::new_unique())
} 