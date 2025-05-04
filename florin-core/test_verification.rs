use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use florin_core::proof_import::{ImportableProof, ProofType, ProofMetadata};
use florin_core::proof_verification::{verify_proof, VerificationConfig, is_proof_valid};

fn main() -> Result<()> {
    println!("Running basic verification tests...");
    
    // Create a valid proof for testing
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
    let result = verify_proof(&valid_proof, &config)?;
    println!("Valid proof verification result: {}", result.is_valid);
    
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
    
    println!("Basic verification tests completed.");
    
    Ok(())
} 