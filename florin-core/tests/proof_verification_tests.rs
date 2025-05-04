use florin_core::proof_import::{ImportableProof, ProofType, ProofMetadata};
use florin_core::proof_verification::{
    verify_proof, VerificationConfig, ProofVerificationError, is_proof_valid
};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper function to create a valid transfer proof for testing
    fn create_valid_transfer_proof() -> ImportableProof {
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
    
    // Helper function to create a valid withdraw proof for testing
    fn create_valid_withdraw_proof() -> ImportableProof {
        ImportableProof {
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
        }
    }

    #[test]
    fn test_valid_transfer_proof() {
        let proof = create_valid_transfer_proof();
        let config = VerificationConfig::default();
        
        let result = verify_proof(&proof, &config).expect("Verification should succeed");
        assert!(result.is_valid);
        
        // Also test the convenience function
        assert!(is_proof_valid(&proof));
    }
    
    #[test]
    fn test_valid_withdraw_proof() {
        let proof = create_valid_withdraw_proof();
        let config = VerificationConfig::default();
        
        let result = verify_proof(&proof, &config).expect("Verification should succeed");
        assert!(result.is_valid);
        
        // Also test the convenience function
        assert!(is_proof_valid(&proof));
    }
    
    #[test]
    fn test_expired_proof() {
        let mut proof = create_valid_transfer_proof();
        
        // Set timestamp to one day ago
        proof.metadata.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 24 * 60 * 60;
        
        // Set expiration to 1 hour (3600 seconds)
        let config = VerificationConfig {
            max_proof_age_seconds: 3600,
            verify_crypto: true,
        };
        
        let result = verify_proof(&proof, &config);
        assert!(result.is_err());
        
        match result {
            Err(ProofVerificationError::ExpiredProof) => (),
            _ => panic!("Expected ExpiredProof error"),
        }
        
        // Convenience function should return false
        assert!(!is_proof_valid(&proof));
    }
    
    #[test]
    fn test_wrong_proof_type() {
        let mut proof = create_valid_transfer_proof();
        
        // Change to an unsupported proof type
        proof.proof_type = ProofType::CiphertextValidity;
        
        let config = VerificationConfig::default();
        let result = verify_proof(&proof, &config);
        
        assert!(result.is_err());
        match result {
            Err(ProofVerificationError::InvalidProofType) => (),
            _ => panic!("Expected InvalidProofType error"),
        }
        
        // Convenience function should return false
        assert!(!is_proof_valid(&proof));
    }
    
    #[test]
    fn test_missing_metadata_transfer() {
        let mut proof = create_valid_transfer_proof();
        
        // Missing source pubkey
        proof.metadata.source_pubkey = None;
        
        let config = VerificationConfig::default();
        let result = verify_proof(&proof, &config);
        
        assert!(result.is_err());
        match result {
            Err(ProofVerificationError::MissingMetadata(_)) => (),
            _ => panic!("Expected MissingMetadata error"),
        }
        
        // Reset and test missing destination pubkey
        let mut proof = create_valid_transfer_proof();
        proof.metadata.destination_pubkey = None;
        
        let result = verify_proof(&proof, &config);
        assert!(result.is_err());
        
        // Reset and test missing amount
        let mut proof = create_valid_transfer_proof();
        proof.metadata.amount = None;
        
        let result = verify_proof(&proof, &config);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_missing_metadata_withdraw() {
        let mut proof = create_valid_withdraw_proof();
        
        // Missing source pubkey
        proof.metadata.source_pubkey = None;
        
        let config = VerificationConfig::default();
        let result = verify_proof(&proof, &config);
        
        assert!(result.is_err());
        match result {
            Err(ProofVerificationError::MissingMetadata(_)) => (),
            _ => panic!("Expected MissingMetadata error"),
        }
        
        // Reset and test missing amount
        let mut proof = create_valid_withdraw_proof();
        proof.metadata.amount = None;
        
        let result = verify_proof(&proof, &config);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_empty_proof_data() {
        let mut proof = create_valid_transfer_proof();
        
        // Empty proof data
        proof.data = vec![];
        
        let config = VerificationConfig::default();
        let result = verify_proof(&proof, &config);
        
        assert!(result.is_err());
        match result {
            Err(ProofVerificationError::InvalidStructure(_)) => (),
            _ => panic!("Expected InvalidStructure error"),
        }
    }
    
    #[test]
    fn test_too_small_proof_data() {
        let mut proof = create_valid_transfer_proof();
        
        // Too small proof data (less than 64 bytes)
        proof.data = vec![1; 32];
        
        let config = VerificationConfig::default();
        let result = verify_proof(&proof, &config);
        
        assert!(result.is_err());
        match result {
            Err(ProofVerificationError::InvalidStructure(_)) => (),
            _ => panic!("Expected InvalidStructure error"),
        }
    }
    
    #[test]
    fn test_verification_config() {
        let proof = create_valid_transfer_proof();
        
        // Test with default config
        let config = VerificationConfig::default();
        assert_eq!(config.max_proof_age_seconds, 3600); // 1 hour by default
        assert!(config.verify_crypto);
        
        // Test with custom config
        let custom_config = VerificationConfig {
            max_proof_age_seconds: 7200, // 2 hours
            verify_crypto: false,
        };
        
        let result = verify_proof(&proof, &custom_config).expect("Verification should succeed");
        assert!(result.is_valid);
    }
} 