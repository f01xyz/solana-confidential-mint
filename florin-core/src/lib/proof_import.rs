use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::path::Path;

/// Represents an importable ZK proof (should match the structure in florin-zk)
#[derive(Serialize, Deserialize, Clone)]
pub struct ImportableProof {
    pub proof_type: ProofType,
    pub data: Vec<u8>,
    pub metadata: ProofMetadata,
}

/// Types of proofs that can be imported
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ProofType {
    Transfer,
    Withdraw,
    CiphertextValidity,
    Range,
}

/// Metadata for the proof
#[derive(Serialize, Deserialize, Clone)]
pub struct ProofMetadata {
    pub source_pubkey: Option<String>,
    pub destination_pubkey: Option<String>,
    pub amount: Option<u64>,
    pub timestamp: u64,
}

/// Import a proof from a file
pub fn import_proof_from_file(path: &Path) -> Result<ImportableProof> {
    let file_content = std::fs::read_to_string(path)?;
    let proof: ImportableProof = serde_json::from_str(&file_content)?;
    Ok(proof)
}

/// Import a proof from a file and verify it
/// This is a convenience function that combines import and verification
pub fn import_and_verify_proof(path: &Path) -> Result<ImportableProof> {
    let proof = import_proof_from_file(path)?;
    
    // Use our new verification module to verify the proof
    if !crate::proof_verification::is_proof_valid(&proof) {
        return Err(anyhow!("Proof verification failed"));
    }
    
    Ok(proof)
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
                proof.metadata.source_pubkey,
                proof.metadata.destination_pubkey,
                proof.metadata.amount);
            
            // In a real implementation, we would use this proof in a transfer transaction
            // e.g., confidential_ops::transfer_ct_with_proof(...)
        },
        ProofType::Withdraw => {
            println!("Using withdraw proof from account {:?} for amount {:?}",
                proof.metadata.source_pubkey,
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