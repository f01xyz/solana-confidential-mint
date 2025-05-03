use anyhow::{anyhow, Result};
use rand::rngs::OsRng;
use solana_zk_token_sdk::encryption::{
    auth_encryption::AeKey,
    elgamal::{ElGamalKeypair, ElGamalPubkey},
};
use spl_token_confidential_transfer_proof_api::{
    transfer::TransferProofData,
    withdraw::WithdrawProofData,
    encryption::ProofData
};

/// Generate a new ElGamal key pair for confidential transfers
pub fn generate_elgamal_keypair() -> ElGamalKeypair {
    ElGamalKeypair::new_rand()
}

/// Generate a new AES key for confidential transfers
pub fn generate_ae_key() -> AeKey {
    AeKey::new_rand()
}

/// Convert ElGamal public key to bytes
pub fn elgamal_pubkey_to_bytes(pubkey: &ElGamalPubkey) -> [u8; 32] {
    pubkey.to_bytes()
}

/// Generate a transfer proof for confidential transfers
/// 
/// # Arguments
/// * `amount` - The amount to transfer
/// * `source_keypair` - The ElGamal keypair of the sender
/// * `destination_pubkey` - The ElGamal public key of the recipient
/// * `auditor_pubkey` - Optional auditor ElGamal public key
pub fn generate_transfer_proof(
    amount: u64,
    source_keypair: &ElGamalKeypair,
    destination_pubkey: &ElGamalPubkey,
    auditor_pubkey: Option<&ElGamalPubkey>,
) -> Result<TransferProofData> {
    TransferProofData::new(
        amount,
        source_keypair,
        *destination_pubkey,
        auditor_pubkey.cloned(),
    ).map_err(|e| anyhow!("Transfer proof generation error: {}", e))
}

/// Generate a withdraw proof for converting confidential tokens to public
/// 
/// # Arguments
/// * `amount` - The amount to withdraw
/// * `keypair` - The ElGamal keypair of the token account
/// * `auditor_pubkey` - Optional auditor ElGamal public key
pub fn generate_withdraw_proof(
    amount: u64,
    keypair: &ElGamalKeypair,
    auditor_pubkey: Option<&ElGamalPubkey>,
) -> Result<WithdrawProofData> {
    WithdrawProofData::new(
        amount,
        keypair,
        auditor_pubkey.cloned(),
    ).map_err(|e| anyhow!("Withdraw proof generation error: {}", e))
}

/// Generate an ElGamal public key validity proof
/// 
/// # Arguments
/// * `keypair` - The ElGamal keypair to generate proof for
pub fn generate_pubkey_validity_proof(
    keypair: &ElGamalKeypair,
) -> Result<ProofData> {
    ProofData::new(keypair)
        .map_err(|e| anyhow!("Public key validity proof generation error: {}", e))
}

/// Verify a transfer proof
/// 
/// # Arguments
/// * `proof_data` - The transfer proof data to verify
pub fn verify_transfer_proof(proof_data: &TransferProofData) -> Result<bool> {
    // Note: In a real implementation, verification would happen on-chain
    // This function would be used for offline verification or testing
    
    // For now, we'll just do a basic check on the proof data
    let proof_verification = proof_data.equality_proof_data.verify();
    let ciphertext_verification = proof_data.ciphertext_validity_proof_data_with_ciphertext.proof_data.verify();
    let range_verification = proof_data.range_proof_data.verify();
    
    Ok(proof_verification && ciphertext_verification && range_verification)
}

/// Verify a withdraw proof
/// 
/// # Arguments
/// * `proof_data` - The withdraw proof data to verify
pub fn verify_withdraw_proof(proof_data: &WithdrawProofData) -> Result<bool> {
    // Note: In a real implementation, verification would happen on-chain
    // This function would be used for offline verification or testing
    
    // Basic proof verification
    let verification = proof_data.proof_data.verify();
    Ok(verification)
} 