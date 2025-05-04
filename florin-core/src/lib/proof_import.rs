use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use base64::{Engine as _, engine::general_purpose};

/// Types of ZK proofs that can be imported (match florin-zk)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ProofType {
    Transfer,
    Withdraw,
    PubkeyValidity,
    // These are the old types, kept for backward compatibility
    TransferWithProof,
    WithdrawWithProof,
}

/// Metadata for the proof (match florin-zk)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProofMetadata {
    pub source_address: Option<String>,
    pub destination_address: Option<String>,
    pub mint_address: Option<String>,
    pub amount: Option<u64>,
    pub timestamp: String,  // ISO8601 format
}

/// Represents a proof DTO imported from florin-zk
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImportableProof {
    pub version: String,
    pub proof_id: String,   // UUID
    pub proof_type: ProofType,
    pub zk_sdk_version: String,
    pub data: String, // Base64 encoded serialized proof data
    pub metadata: ProofMetadata,
}

/// Custom wrapper for proof data types
/// This must match the format used in florin-zk
#[derive(Serialize, Deserialize)]
struct SerializableProofData {
    pub data_type: String,
    pub binary_data: Vec<u8>,
}

/// Import a proof from a file
pub fn import_proof_from_file(path: &Path) -> Result<ImportableProof> {
    let file_content = fs::read_to_string(path)?;
    let proof: ImportableProof = serde_json::from_str(&file_content)
        .map_err(|e| anyhow!("Failed to parse proof JSON: {}", e))?;
    Ok(proof)
}

/// Import a proof from a file and verify it
/// This is a convenience function that combines import and verification
pub fn import_and_verify_proof(path: &Path) -> Result<ImportableProof> {
    let proof = import_proof_from_file(path)?;
    
    // Use our verification module to verify the proof
    if !crate::proof_verification::is_proof_valid(&proof) {
        return Err(anyhow!("Proof verification failed"));
    }
    
    Ok(proof)
}

/// Extract the binary proof data from the base64 encoded data field
pub fn extract_proof_binary_data(proof: &ImportableProof) -> Result<Vec<u8>> {
    // Decode the base64 data
    let decoded = general_purpose::STANDARD.decode(&proof.data)
        .map_err(|e| anyhow!("Failed to decode base64 data: {}", e))?;
    
    // Parse the inner serialized proof data
    let serializable_proof: SerializableProofData = serde_json::from_slice(&decoded)
        .map_err(|e| anyhow!("Failed to parse inner proof data: {}", e))?;
    
    Ok(serializable_proof.binary_data)
}

/// Check if the proof version is compatible with the current implementation
pub fn is_version_compatible(proof: &ImportableProof) -> bool {
    // Parse version components
    let version_parts: Vec<&str> = proof.version.split('.').collect();
    if version_parts.len() != 3 {
        return false;
    }
    
    // For now we only care about major version compatibility
    // In a real implementation, this would be more sophisticated
    match version_parts[0].parse::<u32>() {
        Ok(major) => major == 1, // We only support major version 1 for now
        Err(_) => false,
    }
}

/// Validate a proof before using it
pub fn validate_proof(proof: &ImportableProof) -> Result<()> {
    // Check version compatibility
    if !is_version_compatible(proof) {
        return Err(anyhow!("Incompatible proof version: {}", proof.version));
    }
    
    // Check that we have the required metadata for this proof type
    match proof.proof_type {
        ProofType::Transfer | ProofType::TransferWithProof => {
            if proof.metadata.source_address.is_none() {
                return Err(anyhow!("Missing source address in transfer proof"));
            }
            if proof.metadata.destination_address.is_none() {
                return Err(anyhow!("Missing destination address in transfer proof"));
            }
            if proof.metadata.amount.is_none() {
                return Err(anyhow!("Missing amount in transfer proof"));
            }
        },
        ProofType::Withdraw | ProofType::WithdrawWithProof => {
            if proof.metadata.source_address.is_none() {
                return Err(anyhow!("Missing source address in withdraw proof"));
            }
            if proof.metadata.amount.is_none() {
                return Err(anyhow!("Missing amount in withdraw proof"));
            }
        },
        _ => {}
    }
    
    Ok(())
}

/// Use the imported proof in a confidential transfer operation
/// This demonstrates how florin-core would use proofs from florin-zk
pub fn use_imported_proof(proof: &ImportableProof) -> Result<()> {
    // In a real implementation, this would use the proof in an on-chain transaction
    
    // First verify the proof
    let config = crate::proof_verification::VerificationConfig::default();
    let verification_result = crate::proof_verification::verify_proof(proof, &config)
        .map_err(|e| anyhow!("Proof verification failed: {}", e))?;
    
    if !verification_result.is_valid {
        return Err(anyhow!("Invalid proof"));
    }
    
    match proof.proof_type {
        ProofType::Transfer => {
            println!("Using transfer proof from {:?} to {:?} for amount {:?}",
                proof.metadata.source_address,
                proof.metadata.destination_address,
                proof.metadata.amount);
            
            // In a real implementation, we would use this proof in a transfer transaction
            // e.g., confidential_ops::transfer_ct_with_proof(...)
        },
        ProofType::Withdraw => {
            println!("Using withdraw proof from account {:?} for amount {:?}",
                proof.metadata.source_address,
                proof.metadata.amount);
            
            // In a real implementation, we would use this proof in a withdraw transaction
            // e.g., confidential_ops::withdraw_ct_with_proof(...)
        },
        _ => {
            println!("Unsupported proof type");
        }
    }
    
    Ok(())
} 