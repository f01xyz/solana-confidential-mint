use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
enum ProofType {
    Transfer,
    Withdraw,
    CiphertextValidity,
    Range,
}

#[derive(Serialize, Deserialize, Clone)]
struct ProofMetadata {
    source_pubkey: Option<String>,
    destination_pubkey: Option<String>,
    amount: Option<u64>,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct ImportableProof {
    proof_type: ProofType,
    data: Vec<u8>,
    metadata: ProofMetadata,
}

#[derive(Debug)]
enum ProofVerificationError {
    InvalidStructure(&'static str),
    ExpiredProof,
    MissingMetadata(&'static str),
    InvalidProofType,
    VerificationFailed,
}

impl std::fmt::Display for ProofVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStructure(msg) => write!(f, "Invalid proof structure: {}", msg),
            Self::ExpiredProof => write!(f, "Proof has expired"),
            Self::MissingMetadata(field) => write!(f, "Missing required metadata: {}", field),
            Self::InvalidProofType => write!(f, "Unsupported proof type"),
            Self::VerificationFailed => write!(f, "Cryptographic verification failed"),
        }
    }
}

struct VerificationConfig {
    max_proof_age_seconds: u64,
    verify_crypto: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            max_proof_age_seconds: 3600, // 1 hour by default
            verify_crypto: true,
        }
    }
}

struct VerificationResult {
    is_valid: bool,
    details: Option<String>,
}

fn verify_transfer_proof(proof: &ImportableProof) -> Result<VerificationResult, ProofVerificationError> {
    // Check required metadata
    if proof.metadata.source_pubkey.is_none() {
        return Err(ProofVerificationError::MissingMetadata("source_pubkey"));
    }
    if proof.metadata.destination_pubkey.is_none() {
        return Err(ProofVerificationError::MissingMetadata("destination_pubkey"));
    }
    if proof.metadata.amount.is_none() {
        return Err(ProofVerificationError::MissingMetadata("amount"));
    }

    // For now, just check that the proof data is non-empty
    if proof.data.is_empty() {
        return Err(ProofVerificationError::InvalidStructure("empty proof data"));
    }
    
    // Example: Check the structure expected for a transfer proof
    if proof.data.len() < 64 {
        return Err(ProofVerificationError::InvalidStructure("proof data too small"));
    }
    
    Ok(VerificationResult {
        is_valid: true,
        details: Some(format!(
            "Verified transfer of {} from {} to {}", 
            proof.metadata.amount.unwrap(),
            proof.metadata.source_pubkey.as_ref().unwrap(),
            proof.metadata.destination_pubkey.as_ref().unwrap()
        )),
    })
}

fn verify_withdraw_proof(proof: &ImportableProof) -> Result<VerificationResult, ProofVerificationError> {
    // Check required metadata
    if proof.metadata.source_pubkey.is_none() {
        return Err(ProofVerificationError::MissingMetadata("source_pubkey"));
    }
    if proof.metadata.amount.is_none() {
        return Err(ProofVerificationError::MissingMetadata("amount"));
    }

    // For now, just check that the proof data is non-empty
    if proof.data.is_empty() {
        return Err(ProofVerificationError::InvalidStructure("empty proof data"));
    }
    
    // Example: Check the structure expected for a withdraw proof
    if proof.data.len() < 64 {
        return Err(ProofVerificationError::InvalidStructure("proof data too small"));
    }
    
    Ok(VerificationResult {
        is_valid: true,
        details: Some(format!(
            "Verified withdrawal of {} from {}", 
            proof.metadata.amount.unwrap(),
            proof.metadata.source_pubkey.as_ref().unwrap()
        )),
    })
}

fn is_proof_expired(proof: &ImportableProof, config: &VerificationConfig) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let proof_age = now.saturating_sub(proof.metadata.timestamp);
    proof_age > config.max_proof_age_seconds
}

fn verify_proof(
    proof: &ImportableProof, 
    config: &VerificationConfig
) -> Result<VerificationResult, ProofVerificationError> {
    // Check if proof has expired
    if is_proof_expired(proof, config) {
        return Err(ProofVerificationError::ExpiredProof);
    }
    
    // Dispatch to the correct verification function based on proof type
    match proof.proof_type {
        ProofType::Transfer => verify_transfer_proof(proof),
        ProofType::Withdraw => verify_withdraw_proof(proof),
        // Add handlers for other proof types as needed
        _ => Err(ProofVerificationError::InvalidProofType),
    }
}

fn is_proof_valid(proof: &ImportableProof) -> bool {
    let config = VerificationConfig::default();
    verify_proof(proof, &config).map(|r| r.is_valid).unwrap_or(false)
}

fn main() {
    println!("Running standalone verification tests...");
    
    // Create a valid transfer proof for testing
    let valid_proof = ImportableProof {
        proof_type: ProofType::Transfer,
        data: vec![1; 100], // 100 bytes of dummy data
        metadata: ProofMetadata {
            source_pubkey: Some("source_pubkey".to_string()),
            destination_pubkey: Some("destination_pubkey".to_string()),
            amount: Some(100),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    };
    
    // Test with default config
    let config = VerificationConfig::default();
    let result = verify_proof(&valid_proof, &config).unwrap();
    println!("Valid proof verification result: {}", result.is_valid);
    println!("Details: {}", result.details.unwrap());
    
    // Create an expired proof
    let mut expired_proof = valid_proof.clone();
    expired_proof.metadata.timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() - 24 * 60 * 60; // 1 day ago
    
    // Test with expired proof
    let expired_result = verify_proof(&expired_proof, &config);
    match expired_result {
        Ok(_) => println!("ERROR: Expired proof was accepted!"),
        Err(e) => println!("Expired proof correctly rejected: {}", e),
    }
    
    // Create a proof with missing metadata
    let mut missing_metadata_proof = valid_proof.clone();
    missing_metadata_proof.metadata.source_pubkey = None;
    
    // Test with missing metadata
    let missing_metadata_result = verify_proof(&missing_metadata_proof, &config);
    match missing_metadata_result {
        Ok(_) => println!("ERROR: Proof with missing metadata was accepted!"),
        Err(e) => println!("Proof with missing metadata correctly rejected: {}", e),
    }
    
    // Create a proof with empty data
    let mut empty_data_proof = valid_proof.clone();
    empty_data_proof.data = vec![];
    
    // Test with empty data
    let empty_data_result = verify_proof(&empty_data_proof, &config);
    match empty_data_result {
        Ok(_) => println!("ERROR: Proof with empty data was accepted!"),
        Err(e) => println!("Proof with empty data correctly rejected: {}", e),
    }
    
    // Create a proof with wrong type
    let mut wrong_type_proof = valid_proof.clone();
    wrong_type_proof.proof_type = ProofType::CiphertextValidity;
    
    // Test with wrong type
    let wrong_type_result = verify_proof(&wrong_type_proof, &config);
    match wrong_type_result {
        Ok(_) => println!("ERROR: Proof with wrong type was accepted!"),
        Err(e) => println!("Proof with wrong type correctly rejected: {}", e),
    }
    
    // Test valid withdraw proof
    let valid_withdraw_proof = ImportableProof {
        proof_type: ProofType::Withdraw,
        data: vec![1; 100], // 100 bytes of dummy data
        metadata: ProofMetadata {
            source_pubkey: Some("source_pubkey".to_string()),
            destination_pubkey: None,
            amount: Some(100),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    };
    
    let withdraw_result = verify_proof(&valid_withdraw_proof, &config).unwrap();
    println!("Valid withdraw proof verification result: {}", withdraw_result.is_valid);
    println!("Details: {}", withdraw_result.details.unwrap());
    
    println!("Standalone verification tests completed.");
} 