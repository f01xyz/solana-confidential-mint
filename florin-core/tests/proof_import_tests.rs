use florin_core::proof_import::{
    import_proof_from_file, import_and_verify_proof, ImportableProof, ProofType, ProofMetadata
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper function to create a valid proof for testing
    fn create_valid_proof() -> ImportableProof {
        ImportableProof {
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
        }
    }
    
    #[test]
    fn test_import_valid_proof() {
        // Create a temporary directory for test files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("valid_proof.json");
        
        // Create a valid proof and serialize to file
        let valid_proof = create_valid_proof();
        let json = serde_json::to_string_pretty(&valid_proof).expect("Failed to serialize proof");
        
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(json.as_bytes()).expect("Failed to write to file");
        
        // Test importing the proof
        let imported_proof = import_proof_from_file(&file_path).expect("Failed to import proof");
        
        // Verify the imported proof matches the original
        assert_eq!(imported_proof.proof_type, valid_proof.proof_type);
        assert_eq!(imported_proof.data, valid_proof.data);
        assert_eq!(imported_proof.metadata.source_pubkey, valid_proof.metadata.source_pubkey);
        assert_eq!(imported_proof.metadata.destination_pubkey, valid_proof.metadata.destination_pubkey);
        assert_eq!(imported_proof.metadata.amount, valid_proof.metadata.amount);
        assert_eq!(imported_proof.metadata.timestamp, valid_proof.metadata.timestamp);
        
        // Test importing and verifying the proof
        let verified_proof = import_and_verify_proof(&file_path).expect("Failed to import and verify proof");
        
        // Verify the verified proof matches the original
        assert_eq!(verified_proof.proof_type, valid_proof.proof_type);
    }
    
    #[test]
    fn test_import_invalid_json() {
        // Create a temporary directory for test files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("invalid_json.json");
        
        // Create an invalid JSON file
        let invalid_json = r#"{ "this is not valid json": "#;
        
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(invalid_json.as_bytes()).expect("Failed to write to file");
        
        // Test importing the invalid JSON
        let result = import_proof_from_file(&file_path);
        assert!(result.is_err(), "Should fail with invalid JSON");
        
        // Test importing and verifying the invalid JSON
        let result = import_and_verify_proof(&file_path);
        assert!(result.is_err(), "Should fail with invalid JSON");
    }
    
    #[test]
    fn test_import_invalid_proof_structure() {
        // Create a temporary directory for test files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("invalid_structure.json");
        
        // Create a JSON file with the wrong structure
        let invalid_structure = r#"{
            "proof_type": "Transfer",
            "data": [1, 2, 3],
            "metadata": {
                "timestamp": 1234567890
            }
        }"#;
        
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(invalid_structure.as_bytes()).expect("Failed to write to file");
        
        // Test importing and verifying the proof with invalid structure
        let result = import_and_verify_proof(&file_path);
        assert!(result.is_err(), "Should fail with invalid proof structure");
    }
    
    #[test]
    fn test_import_nonexistent_file() {
        // Create a path to a file that doesn't exist
        let file_path = PathBuf::from("nonexistent_file.json");
        
        // Test importing a nonexistent file
        let result = import_proof_from_file(&file_path);
        assert!(result.is_err(), "Should fail with nonexistent file");
        
        // Test importing and verifying a nonexistent file
        let result = import_and_verify_proof(&file_path);
        assert!(result.is_err(), "Should fail with nonexistent file");
    }
    
    #[test]
    fn test_import_expired_proof() {
        // Create a temporary directory for test files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("expired_proof.json");
        
        // Create a proof with an old timestamp
        let mut expired_proof = create_valid_proof();
        expired_proof.metadata.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 24 * 60 * 60; // 1 day ago
        
        let json = serde_json::to_string_pretty(&expired_proof).expect("Failed to serialize proof");
        
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(json.as_bytes()).expect("Failed to write to file");
        
        // Test importing and verifying the expired proof
        // This should fail because the default expiration is 1 hour
        let result = import_and_verify_proof(&file_path);
        assert!(result.is_err(), "Should fail with expired proof");
    }
    
    #[test]
    fn test_import_empty_proof_data() {
        // Create a temporary directory for test files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("empty_data_proof.json");
        
        // Create a proof with empty data
        let mut empty_data_proof = create_valid_proof();
        empty_data_proof.data = vec![];
        
        let json = serde_json::to_string_pretty(&empty_data_proof).expect("Failed to serialize proof");
        
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(json.as_bytes()).expect("Failed to write to file");
        
        // Test importing and verifying the proof with empty data
        let result = import_and_verify_proof(&file_path);
        assert!(result.is_err(), "Should fail with empty proof data");
    }
} 