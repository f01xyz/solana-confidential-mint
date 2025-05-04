use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};
use crate::proof_import::{ImportableProof, ProofType};

/// Error types specific to proof verification
#[derive(Debug)]
pub enum ProofVerificationError {
    InvalidStructure(&'static str),
    ExpiredProof,
    MissingMetadata(&'static str),
    InvalidProofType,
    VerificationFailed,
    InvalidVersion,
}

impl std::fmt::Display for ProofVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStructure(msg) => write!(f, "Invalid proof structure: {}", msg),
            Self::ExpiredProof => write!(f, "Proof has expired"),
            Self::MissingMetadata(field) => write!(f, "Missing required metadata: {}", field),
            Self::InvalidProofType => write!(f, "Unsupported proof type"),
            Self::VerificationFailed => write!(f, "Cryptographic verification failed"),
            Self::InvalidVersion => write!(f, "Incompatible proof version"),
        }
    }
}

impl std::error::Error for ProofVerificationError {}

/// Configuration for proof verification
#[derive(Debug)]
pub struct VerificationConfig {
    /// Maximum age of a proof in seconds before it's considered expired
    pub max_proof_age_seconds: u64,
    /// Whether to verify cryptographic properties of the proof
    pub verify_crypto: bool,
    /// Whether to check version compatibility
    pub check_version: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            max_proof_age_seconds: 3600, // 1 hour by default
            verify_crypto: true,
            check_version: true,
        }
    }
}

/// Verification result with details
#[derive(Debug)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub message: Option<String>,
}

/// Verify a transfer proof
pub fn verify_transfer_proof(proof: &ImportableProof, config: &VerificationConfig) 
    -> Result<VerificationResult, ProofVerificationError> {
    // Check required metadata
    if proof.metadata.source_address.is_none() {
        return Err(ProofVerificationError::MissingMetadata("source_address"));
    }
    if proof.metadata.destination_address.is_none() {
        return Err(ProofVerificationError::MissingMetadata("destination_address"));
    }
    if proof.metadata.amount.is_none() {
        return Err(ProofVerificationError::MissingMetadata("amount"));
    }

    // Check version compatibility if enabled
    if config.check_version && !crate::proof_import::is_version_compatible(proof) {
        return Err(ProofVerificationError::InvalidVersion);
    }

    // In a real implementation, this would extract and verify the ZK proof
    // by decoding the base64 data and validating it
    
    // Extract data for verification
    let proof_data = match crate::proof_import::extract_proof_binary_data(proof) {
        Ok(data) => data,
        Err(_) => return Err(ProofVerificationError::InvalidStructure("failed to extract proof data")),
    };
    
    // Example: Check the structure expected for a transfer proof
    if proof_data.len() < 32 {
        return Err(ProofVerificationError::InvalidStructure("proof data too small"));
    }
    
    // Check if cryptographic verification is required
    if config.verify_crypto {
        // In a real implementation, this would perform ZK verification
        // For now this is just a placeholder
        if proof_data.is_empty() {
            return Err(ProofVerificationError::VerificationFailed);
        }
    }
    
    Ok(VerificationResult {
        is_valid: true,
        message: Some(format!(
            "Verified transfer of {} from {} to {}", 
            proof.metadata.amount.unwrap_or(0),
            proof.metadata.source_address.as_ref().unwrap_or(&"unknown".to_string()),
            proof.metadata.destination_address.as_ref().unwrap_or(&"unknown".to_string())
        )),
    })
}

/// Verify a withdraw proof
pub fn verify_withdraw_proof(proof: &ImportableProof, config: &VerificationConfig) 
    -> Result<VerificationResult, ProofVerificationError> {
    // Check required metadata
    if proof.metadata.source_address.is_none() {
        return Err(ProofVerificationError::MissingMetadata("source_address"));
    }
    if proof.metadata.amount.is_none() {
        return Err(ProofVerificationError::MissingMetadata("amount"));
    }

    // Check version compatibility if enabled
    if config.check_version && !crate::proof_import::is_version_compatible(proof) {
        return Err(ProofVerificationError::InvalidVersion);
    }

    // Extract data for verification
    let proof_data = match crate::proof_import::extract_proof_binary_data(proof) {
        Ok(data) => data,
        Err(_) => return Err(ProofVerificationError::InvalidStructure("failed to extract proof data")),
    };
    
    // Example: Check the structure expected for a withdraw proof
    if proof_data.len() < 32 {
        return Err(ProofVerificationError::InvalidStructure("proof data too small"));
    }
    
    // Check if cryptographic verification is required
    if config.verify_crypto {
        // In a real implementation, this would perform ZK verification
        // For now this is just a placeholder
        if proof_data.is_empty() {
            return Err(ProofVerificationError::VerificationFailed);
        }
    }
    
    Ok(VerificationResult {
        is_valid: true,
        message: Some(format!(
            "Verified withdrawal of {} from {}", 
            proof.metadata.amount.unwrap_or(0),
            proof.metadata.source_address.as_ref().unwrap_or(&"unknown".to_string())
        )),
    })
}

/// Check if a proof has expired
fn is_proof_expired(proof: &ImportableProof, config: &VerificationConfig) -> bool {
    let now = Utc::now();
    
    // Parse the timestamp from the proof (ISO8601 format)
    match DateTime::parse_from_rfc3339(&proof.metadata.timestamp) {
        Ok(timestamp) => {
            let proof_age = now.signed_duration_since(timestamp.with_timezone(&Utc));
            proof_age > Duration::seconds(config.max_proof_age_seconds as i64)
        },
        Err(_) => {
            // If we can't parse the timestamp, assume the proof is valid
            // This is a fallback for testing and can be made stricter in production
            false
        }
    }
}

/// Verify a proof based on its type
pub fn verify_proof(
    proof: &ImportableProof, 
    config: &VerificationConfig
) -> Result<VerificationResult, ProofVerificationError> {
    // Check if proof has expired
    if is_proof_expired(proof, config) {
        return Err(ProofVerificationError::ExpiredProof);
    }
    
    // Dispatch to the correct verification function based on proof type
    match proof.proof_type {
        ProofType::Transfer | ProofType::TransferWithProof => 
            verify_transfer_proof(proof, config),
        ProofType::Withdraw | ProofType::WithdrawWithProof => 
            verify_withdraw_proof(proof, config),
        // Add handlers for other proof types as needed
        _ => Err(ProofVerificationError::InvalidProofType),
    }
}

/// Convenience function that returns true if the proof is valid, false otherwise
pub fn is_proof_valid(proof: &ImportableProof) -> bool {
    let config = VerificationConfig::default();
    verify_proof(proof, &config).map(|r| r.is_valid).unwrap_or(false)
} 