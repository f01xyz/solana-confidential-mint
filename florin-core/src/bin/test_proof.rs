use std::path::Path;
use anyhow::{Result, Context};
use florin_core::proof_import;
use florin_core::proof_verification;

fn main() -> Result<()> {
    println!("Testing proof import and validation...");
    
    let proof_path = Path::new("../test-data/sample_transfer_proof.json");
    
    // Import the proof
    let proof = proof_import::import_proof_from_file(proof_path)
        .context("Failed to import proof")?;
    
    println!("✅ Successfully imported proof");
    println!("Proof type: {:?}", proof.proof_type);
    println!("Source: {:?}", proof.metadata.source_address);
    println!("Destination: {:?}", proof.metadata.destination_address);
    println!("Amount: {:?}", proof.metadata.amount);
    
    // Validate the proof
    match proof_import::validate_proof(&proof) {
        Ok(_) => println!("✅ Proof structure validation passed"),
        Err(e) => println!("❌ Proof structure validation failed: {}", e),
    }
    
    // Verify the proof
    let config = proof_verification::VerificationConfig::default();
    match proof_verification::verify_proof(&proof, &config) {
        Ok(result) => {
            if result.is_valid {
                println!("✅ Proof verification passed: {}", 
                    result.message.unwrap_or_else(|| "No details".to_string()));
            } else {
                println!("❌ Proof verification failed");
            }
        },
        Err(e) => println!("❌ Proof verification error: {}", e),
    }
    
    println!("Would extract proof data for on-chain use:");
    match proof_import::extract_proof_binary_data(&proof) {
        Ok(data) => println!("✅ Successfully extracted {} bytes of proof data", data.len()),
        Err(e) => println!("❌ Failed to extract proof data: {}", e),
    }
    
    println!("Test complete!");
    Ok(())
}
