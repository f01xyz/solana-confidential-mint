use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::zk_proofs;
use solana_zk_token_sdk::encryption::elgamal::{ElGamalKeypair, ElGamalPubkey};
use spl_token_confidential_transfer_proof_api::{
    transfer::TransferProofData,
    withdraw::WithdrawProofData,
};
use base64::{Engine as _, engine::general_purpose};

/// Types of ZK proofs that can be exported
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProofType {
    Transfer,
    Withdraw,
    PubkeyValidity,
}

/// Metadata for the proof
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProofMetadata {
    pub source_pubkey: Option<String>,
    pub destination_pubkey: Option<String>,
    pub amount: Option<u64>,
    pub timestamp: u64,
}

/// Represents a serializable ZK proof that can be exported to a file
/// and imported in florin-core
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportableProof {
    pub proof_type: ProofType,
    pub data: String, // Base64 encoded serialized proof data
    pub metadata: ProofMetadata,
}

/// Export a transfer proof to a file
pub fn export_transfer_proof(
    proof_data: &TransferProofData,
    amount: u64,
    source_pubkey: Option<String>,
    destination_pubkey: Option<String>,
    path: &Path,
) -> Result<()> {
    // Serialize the proof data
    let proof_bytes = bincode::serialize(proof_data)?;
    
    // Create the exportable proof
    let exportable_proof = ExportableProof {
        proof_type: ProofType::Transfer,
        data: general_purpose::STANDARD.encode(proof_bytes),
        metadata: ProofMetadata {
            source_pubkey,
            destination_pubkey,
            amount: Some(amount),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    };
    
    // Write to file
    let serialized = serde_json::to_string_pretty(&exportable_proof)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    
    Ok(())
}

/// Export a withdraw proof to a file
pub fn export_withdraw_proof(
    proof_data: &WithdrawProofData,
    amount: u64,
    source_pubkey: Option<String>,
    path: &Path,
) -> Result<()> {
    // Serialize the proof data
    let proof_bytes = bincode::serialize(proof_data)?;
    
    // Create the exportable proof
    let exportable_proof = ExportableProof {
        proof_type: ProofType::Withdraw,
        data: general_purpose::STANDARD.encode(proof_bytes),
        metadata: ProofMetadata {
            source_pubkey,
            destination_pubkey: None,
            amount: Some(amount),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    };
    
    // Write to file
    let serialized = serde_json::to_string_pretty(&exportable_proof)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    
    Ok(())
}

/// Import a proof from a file
pub fn import_proof_from_file(path: &Path) -> Result<ExportableProof> {
    let file_content = std::fs::read_to_string(path)?;
    let proof: ExportableProof = serde_json::from_str(&file_content)?;
    Ok(proof)
}

/// Extract a transfer proof from an imported exportable proof
pub fn extract_transfer_proof(proof: &ExportableProof) -> Result<TransferProofData> {
    if proof.proof_type != ProofType::Transfer {
        return Err(anyhow!("Invalid proof type: expected Transfer proof"));
    }
    
    // Decode base64 and deserialize
    let proof_bytes = general_purpose::STANDARD.decode(&proof.data)?;
    let transfer_proof: TransferProofData = bincode::deserialize(&proof_bytes)?;
    
    Ok(transfer_proof)
}

/// Extract a withdraw proof from an imported exportable proof
pub fn extract_withdraw_proof(proof: &ExportableProof) -> Result<WithdrawProofData> {
    if proof.proof_type != ProofType::Withdraw {
        return Err(anyhow!("Invalid proof type: expected Withdraw proof"));
    }
    
    // Decode base64 and deserialize
    let proof_bytes = general_purpose::STANDARD.decode(&proof.data)?;
    let withdraw_proof: WithdrawProofData = bincode::deserialize(&proof_bytes)?;
    
    Ok(withdraw_proof)
}

/// Create a demo transfer proof and export it to a file
pub fn create_and_export_demo_transfer_proof(
    amount: u64,
    source_keypair: &ElGamalKeypair,
    destination_pubkey: &ElGamalPubkey,
    path: &Path,
) -> Result<()> {
    // Generate the transfer proof
    let transfer_proof = zk_proofs::generate_transfer_proof(
        amount,
        source_keypair,
        destination_pubkey,
        None,
    )?;
    
    // Export the proof
    export_transfer_proof(
        &transfer_proof,
        amount,
        Some(bs58::encode(source_keypair.public.to_bytes()).into_string()),
        Some(bs58::encode(destination_pubkey.to_bytes()).into_string()),
        path,
    )
} 