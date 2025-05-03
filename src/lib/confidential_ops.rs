use anyhow::{anyhow, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
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
    extension::{
        confidential_transfer::{
            self, 
            account_info::{TransferAccountInfo, WithdrawAccountInfo},
            ConfidentialTransferAccount,
            instruction::{configure_account, PubkeyValidityProofData},
        },
        BaseStateWithExtensions, ExtensionType,
    },
    solana_zk_sdk::encryption::{auth_encryption::*, elgamal::*},
};
use spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation};
use spl_token_confidential_transfer_proof_generation::withdraw::WithdrawProofData;
use rand::rngs::OsRng;

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
    let elgamal_keypair = ElGamalKeypair::new_from_rng(&mut OsRng)?;
    
    // Create an AES key for decrypting transfer amount history
    let ae_key = AeKey::new_from_rng(&mut OsRng)?;
    
    // Generate validity proof - proves that the ElGamal public key is well-formed
    let pubkey_validity_proof =
        PubkeyValidityProofData::new(&elgamal_keypair).map_err(|e| anyhow!("Proof generation error: {}", e))?;
    
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
            &token_account,               // Token account to configure
            &elgamal_keypair.public,      // ElGamal public key for encryption
            spl_token_2022::solana_zk_sdk::zk_token_elgamal::pod::ElGamalPubkey::zeroed(), // Decryptable balance
            &pubkey_validity_proof,       // Proof that ElGamal keypair is valid
            &owner.pubkey(),              // Account owner
            Some(&owner.pubkey()),        // Authority to close the account (optional)
            &[owner],                     // Signers
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
    
    // Get the confidential transfer extension data
    let token_account_state = account.data.as_slice();
    let ct_account = BaseStateWithExtensions::<ConfidentialTransferAccount>::unpack(token_account_state)?;
    let ct_state = ct_account.get_extension::<ConfidentialTransferAccount>()?;
    
    // Decrypt balances
    let pending_balance_lo = ct_state.pending_balance_lo;
    let pending_balance_hi = ct_state.pending_balance_hi;
    
    let available_balance = ct_state.available_balance;
    let closable = ct_state.closable;
    
    let mut pending_balance = 0;
    if !pending_balance_lo.is_zero() || !pending_balance_hi.is_zero() {
        let ciphertext = confidential_transfer::EncryptedBalance::new(
            pending_balance_lo,
            pending_balance_hi,
            elgamal_keypair.public.into(),
        );
        
        pending_balance = ciphertext
            .decrypt(&elgamal_keypair.secret)
            .map_err(|e| anyhow!("Decryption error: {}", e))?;
    }
    
    let mut available_amount = 0;
    if !available_balance.is_zero() {
        let ciphertext = available_balance.try_into()?;
        available_amount = ciphertext
            .decrypt(&elgamal_keypair.secret)
            .map_err(|e| anyhow!("Decryption error: {}", e))?;
    }
    
    let mut closing_available_amount = 0;
    if closable > 0 {
        closing_available_amount = confidential_transfer::decrypt_amount_with_ab_shared_km(
            &*ae_key.get_encoding(),
        ).map_err(|e| anyhow!("Decryption error for closing amount: {}", e))?;
    }
    
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
            token_account,         // Token account
            &owner.pubkey(),       // Owner
            amount,                // Amount to deposit
            decimals,              // Decimals
            elgamal_keypair.public, // ElGamal public key (not needed here)
            None,                  // No auditor
            &[owner],              // Signers
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
            token_account,        // Token account
            &owner.pubkey(),      // Owner
            elgamal_keypair,      // ElGamal keypair for decryption
            ae_key,               // AES key for encryption
            None,                 // No auditor
            &[owner],             // Signers
        )
        .await?;
    
    Ok(signature.to_string())
}

/// Transfer tokens confidentially
pub async fn transfer_ct(
    client: Arc<RpcClient>,
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    owner: &Keypair,
    source_elgamal_keypair: &ElGamalKeypair,
    source_ae_key: &AeKey,
    destination_elgamal_pubkey: &ElGamalPubkey,
    amount: u64,
    decimals: u8,
) -> Result<String> {
    // Set up program client for Token client
    let program_client = ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction,
    );
    
    // Get the mint from source token account
    let source_account_info = client.get_account(source_token_account).await?;
    let source_account_data = source_account_info.data.as_slice();
    let source_account = spl_token_2022::state::Account::unpack(source_account_data)?;
    
    // Create token client
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(),
        &source_account.mint,
        Some(decimals),
        Arc::new(owner.insecure_clone()),
    );
    
    // Get source and destination accounts
    let source_info = TransferAccountInfo::new(
        *source_token_account,
        source_elgamal_keypair.clone(),
        source_ae_key.clone(),
    );
    
    // Generate and extract the proofs for confidential transfer
    // We need 3 types of proofs:
    // 1. Equality proof - proves the sum of input amounts equals sum of output amounts
    // 2. Ciphertext validity proof - proves the ciphertext is correctly formed
    // 3. Range proof - proves the amount is within a valid range (non-negative)
    
    // Create keypairs for the proof context state accounts
    let equality_proof_context_state_keypair = Keypair::new();
    let ciphertext_validity_proof_context_state_keypair = Keypair::new();
    let range_proof_context_state_keypair = Keypair::new();
    
    // Generate proofs using the confidential transfer proof generation crate
    let transfer_proof_data = 
        spl_token_confidential_transfer_proof_generation::transfer::TransferProofData::new(
            amount,                    // Amount to transfer
            source_elgamal_keypair,   // Source ElGamal keypair
            *destination_elgamal_pubkey, // Destination ElGamal public key
            None,                    // No auditor
        ).map_err(|e| anyhow!("Proof generation error: {}", e))?;
    
    // Create context state accounts for each proof
    // Equality proof - proves that the transfer amount is correct
    let equality_proof_context_state_pubkey = equality_proof_context_state_keypair.pubkey();
    let equality_proof_signature = token
        .confidential_transfer_create_context_state_account(
            &equality_proof_context_state_pubkey,     // Account to create
            &owner.pubkey(),                          // Payer for the account creation
            &transfer_proof_data.equality_proof_data, // Proof data to verify
            false,                                    // Not a range proof
            &[&equality_proof_context_state_keypair], // Signers
        )
        .await?;
    
    // Ciphertext validity proof - proves that the ciphertext is properly formed
    let ciphertext_validity_proof_context_state_pubkey = ciphertext_validity_proof_context_state_keypair.pubkey();
    let ciphertext_proof_signature = token
        .confidential_transfer_create_context_state_account(
            &ciphertext_validity_proof_context_state_pubkey, // Account to create
            &owner.pubkey(),                                 // Payer for the account creation
            &transfer_proof_data
                .ciphertext_validity_proof_data_with_ciphertext
                .proof_data, // Proof data to verify
            false,                                           // Not a range proof
            &[&ciphertext_validity_proof_context_state_keypair], // Signers
        )
        .await?;
    
    // Range proof - proves that the encrypted amount is within a valid range and non-negative
    let range_proof_context_state_pubkey = range_proof_context_state_keypair.pubkey();
    let range_proof_signature = token
        .confidential_transfer_create_context_state_account(
            &range_proof_context_state_pubkey,     // Account to create
            &owner.pubkey(),                       // Payer for the account creation
            &transfer_proof_data.range_proof_data, // Proof to verify
            true,                                  // Is a range proof
            &[&range_proof_context_state_keypair], // Signers
        )
        .await?;
    
    // Create a ProofAccountWithCiphertext for the ciphertext validity proof
    // This combines the proof account with the ciphertext data
    let ciphertext_validity_proof_account_with_ciphertext =
        spl_token_client::token::ProofAccountWithCiphertext {
            proof_account: spl_token_client::token::ProofAccount::ContextAccount(
                ciphertext_validity_proof_context_state_pubkey, // Proof account
            ),
            ciphertext_lo: transfer_proof_data
                .ciphertext_validity_proof_data_with_ciphertext
                .ciphertext_lo, // Low 128 bits of ciphertext
            ciphertext_hi: transfer_proof_data
                .ciphertext_validity_proof_data_with_ciphertext
                .ciphertext_hi, // High 128 bits of ciphertext
        };
    
    // Perform the confidential transfer
    let transfer_signature = token
        .confidential_transfer_transfer(
            source_token_account,       // Source token account
            destination_token_account,  // Destination token account
            &owner.pubkey(),            // Owner of the source account
            Some(&spl_token_client::token::ProofAccount::ContextAccount(
                equality_proof_context_state_pubkey, // Equality proof context state account
            )),
            Some(&ciphertext_validity_proof_account_with_ciphertext), // Ciphertext validity proof
            Some(&spl_token_client::token::ProofAccount::ContextAccount(
                range_proof_context_state_pubkey, // Range proof account
            )),
            amount,                     // Amount to transfer
            None,                       // Optional auditor info (none in this case)
            source_elgamal_keypair,     // Sender's ElGamal keypair
            source_ae_key,              // Sender's AES key
            destination_elgamal_pubkey, // Recipient's ElGamal public key
            None,                       // Optional auditor ElGamal public key
            &[owner],                   // Signers
        )
        .await?;
    
    // Close the proof context state accounts to recover rent
    token.confidential_transfer_close_context_state_account(
        &equality_proof_context_state_pubkey, // Account to close
        source_token_account,                 // Destination for the rent
        &owner.pubkey(),                      // Authority to close the account
        &[owner],                             // Signers
    ).await?;
    
    token.confidential_transfer_close_context_state_account(
        &ciphertext_validity_proof_context_state_pubkey, // Account to close
        source_token_account,                            // Destination for the rent
        &owner.pubkey(),                                 // Authority to close the account
        &[owner],                                        // Signers
    ).await?;
    
    token.confidential_transfer_close_context_state_account(
        &range_proof_context_state_pubkey, // Account to close
        source_token_account,              // Destination for the rent
        &owner.pubkey(),                   // Authority to close the account
        &[owner],                          // Signers
    ).await?;
    
    Ok(transfer_signature.to_string())
}

/// Withdraw tokens from confidential available balance to public balance
pub async fn withdraw_ct(
    client: Arc<RpcClient>,
    token_account: &Pubkey,
    owner: &Keypair,
    elgamal_keypair: &ElGamalKeypair,
    ae_key: &AeKey,
    amount: u64,
    decimals: u8,
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
        Some(decimals),
        Arc::new(owner.insecure_clone()),
    );
    
    // Create withdraw account info
    let account_info = WithdrawAccountInfo::new(
        *token_account,
        elgamal_keypair.clone(),
        ae_key.clone(),
    );
    
    // Generate the withdraw proof
    let withdraw_proof_data = WithdrawProofData::new(
        amount,
        elgamal_keypair,
        None,
    ).map_err(|e| anyhow!("Withdraw proof generation error: {}", e))?;
    
    // Create a keypair for the proof context state account
    let proof_context_keypair = Keypair::new();
    let proof_context_pubkey = proof_context_keypair.pubkey();
    
    // Create the context state account for the proof
    token.confidential_transfer_create_context_state_account(
        &proof_context_pubkey,                // Account to create
        &owner.pubkey(),                      // Payer for the account creation
        &withdraw_proof_data.proof_data,      // Proof data to verify
        true,                                 // This is a range proof
        &[&proof_context_keypair],            // Signers
    ).await?;
    
    // Perform the withdraw operation
    let signature = token
        .confidential_transfer_withdraw(
            token_account,                    // Token account
            &owner.pubkey(),                  // Owner
            amount,                           // Amount to withdraw
            decimals,                         // Decimals
            Some(&spl_token_client::token::ProofAccount::ContextAccount(
                proof_context_pubkey,         // Proof context state account
            )),
            None,                             // No auditor
            elgamal_keypair,                  // ElGamal keypair
            ae_key,                           // AES key
            &[owner],                         // Signers
        )
        .await?;
    
    // Close the proof context state account to recover rent
    token.confidential_transfer_close_context_state_account(
        &proof_context_pubkey,  // Account to close
        token_account,          // Destination for the rent
        &owner.pubkey(),        // Authority to close the account
        &[owner],               // Signers
    ).await?;
    
    Ok(signature.to_string())
} 