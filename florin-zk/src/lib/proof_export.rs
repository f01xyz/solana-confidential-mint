use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::zk_proofs::{self};
use solana_zk_token_sdk::encryption::elgamal::{ElGamalKeypair, ElGamalPubkey};
use solana_zk_token_sdk::zk_token_elgamal::pod::{AeCiphertext, ElGamalCiphertext};
use bytemuck::{Pod, Zeroable, try_from_bytes, cast_ref};
use spl_token_confidential_transfer_proof_generation::{
    transfer::TransferProofData,
    withdraw::WithdrawProofData,
};
use spl_token_2022::extension::confidential_transfer::instruction::{
    TransferWithFeeInstructionData, WithdrawInstructionData
};
use base64::{Engine as _, engine::general_purpose};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Get current crate version from environment
pub fn get_zk_sdk_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Types of ZK proofs that can be exported
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ProofType {
    Transfer,
    Withdraw,
    PubkeyValidity,
    // These are the old types, kept for backward compatibility
    TransferWithProof,
    WithdrawWithProof,
}

/// Metadata for the proof
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProofMetadata {
    pub source_address: Option<String>,
    pub destination_address: Option<String>,
    pub mint_address: Option<String>,
    pub amount: Option<u64>,
    pub timestamp: String,  // ISO8601 format
}

/// Represents a serializable ZK proof that can be exported to a file
/// and imported in florin-core
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportableProof {
    pub version: String,
    pub proof_id: String,   // UUID
    pub proof_type: ProofType,
    pub zk_sdk_version: String,
    pub data: String, // Base64 encoded serialized proof data
    pub metadata: ProofMetadata,
}

/// Custom wrapper for proof data types
#[derive(Serialize, Deserialize)]
struct SerializableProofData {
    pub data_type: String,
    pub binary_data: Vec<u8>,
}

/// DTO for TransferWithFeeInstructionData to enable serialization
#[derive(Serialize, Deserialize, Debug)]
pub struct TransferWithFeeDto {
    pub new_source_decryptable_available_balance: Vec<u8>,
    pub transfer_amount_auditor_ciphertext_lo: Vec<u8>,
    pub transfer_amount_auditor_ciphertext_hi: Vec<u8>,
    pub equality_proof_instruction_offset: i8,
    pub transfer_amount_ciphertext_validity_proof_instruction_offset: i8,
    pub fee_sigma_proof_instruction_offset: i8,
    pub fee_ciphertext_validity_proof_instruction_offset: i8,
    pub range_proof_instruction_offset: i8,
}

impl From<&TransferWithFeeInstructionData> for TransferWithFeeDto {
    fn from(src: &TransferWithFeeInstructionData) -> Self {
        Self {
            new_source_decryptable_available_balance: bytemuck::bytes_of(&src.new_source_decryptable_available_balance).to_vec(),
            transfer_amount_auditor_ciphertext_lo: bytemuck::bytes_of(&src.transfer_amount_auditor_ciphertext_lo).to_vec(),
            transfer_amount_auditor_ciphertext_hi: bytemuck::bytes_of(&src.transfer_amount_auditor_ciphertext_hi).to_vec(),
            equality_proof_instruction_offset: src.equality_proof_instruction_offset,
            transfer_amount_ciphertext_validity_proof_instruction_offset: src.transfer_amount_ciphertext_validity_proof_instruction_offset,
            fee_sigma_proof_instruction_offset: src.fee_sigma_proof_instruction_offset,
            fee_ciphertext_validity_proof_instruction_offset: src.fee_ciphertext_validity_proof_instruction_offset,
            range_proof_instruction_offset: src.range_proof_instruction_offset,
        }
    }
}

impl TransferWithFeeDto {
    pub fn to_instruction_data(&self) -> TransferWithFeeInstructionData {
        // Create the instruction data struct
        let mut instruction_data = TransferWithFeeInstructionData {
            new_source_decryptable_available_balance: unsafe { std::mem::zeroed() },
            transfer_amount_auditor_ciphertext_lo: unsafe { std::mem::zeroed() },
            transfer_amount_auditor_ciphertext_hi: unsafe { std::mem::zeroed() },
            equality_proof_instruction_offset: self.equality_proof_instruction_offset,
            transfer_amount_ciphertext_validity_proof_instruction_offset: self.transfer_amount_ciphertext_validity_proof_instruction_offset,
            fee_sigma_proof_instruction_offset: self.fee_sigma_proof_instruction_offset,
            fee_ciphertext_validity_proof_instruction_offset: self.fee_ciphertext_validity_proof_instruction_offset,
            range_proof_instruction_offset: self.range_proof_instruction_offset,
        };
        
        // Return the instruction data
        instruction_data
    }
}

/// DTO for WithdrawInstructionData to enable serialization
#[derive(Serialize, Deserialize, Debug)]
pub struct WithdrawDto {
    pub amount: u64,
    pub decimals: u8,
    pub new_decryptable_available_balance: Vec<u8>,
    pub equality_proof_instruction_offset: i8,
    pub range_proof_instruction_offset: i8,
}

impl From<&WithdrawInstructionData> for WithdrawDto {
    fn from(src: &WithdrawInstructionData) -> Self {
        Self {
            amount: u64::from(src.amount),
            decimals: src.decimals,
            new_decryptable_available_balance: bytemuck::bytes_of(&src.new_decryptable_available_balance).to_vec(),
            equality_proof_instruction_offset: src.equality_proof_instruction_offset,
            range_proof_instruction_offset: src.range_proof_instruction_offset,
        }
    }
}

impl WithdrawDto {
    pub fn to_instruction_data(&self) -> WithdrawInstructionData {
        use spl_pod::primitives::PodU64;
        
        // Create the instruction data struct
        let instruction_data = WithdrawInstructionData {
            amount: PodU64::from(self.amount),
            decimals: self.decimals,
            new_decryptable_available_balance: unsafe { std::mem::zeroed() },
            equality_proof_instruction_offset: self.equality_proof_instruction_offset,
            range_proof_instruction_offset: self.range_proof_instruction_offset,
        };
        
        // Return the instruction data
        instruction_data
    }
}

/// Export a transfer proof to a file
pub fn export_transfer_proof(
    _proof_data: &TransferProofData,
    amount: u64,
    source_pubkey: Option<String>,
    destination_pubkey: Option<String>,
    path: &Path,
) -> Result<()> {
    // Since TransferProofData doesn't implement Serialize, we'll create a custom serializable wrapper
    let serializable_proof = SerializableProofData {
        data_type: "transfer_proof".to_string(),
        // In a real implementation we would serialize the proof data properly
        binary_data: vec![0; 64], // Placeholder
    };
    
    // Serialize our wrapper instead
    let serialized_proof = serde_json::to_string(&serializable_proof)?;
    
    // Create the exportable proof
    let exportable_proof = ExportableProof {
        version: "1.0.0".to_string(),
        proof_id: Uuid::new_v4().to_string(),
        proof_type: ProofType::Transfer,
        zk_sdk_version: get_zk_sdk_version(),
        data: general_purpose::STANDARD.encode(serialized_proof),
        metadata: ProofMetadata {
            source_address: source_pubkey,
            destination_address: destination_pubkey,
            mint_address: None, // This should be populated in the new implementation
            amount: Some(amount),
            timestamp: Utc::now().to_rfc3339(),
        },
    };
    
    // Write to file
    let serialized = serde_json::to_string_pretty(&exportable_proof)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    
    Ok(())
}

/// Export a transfer with proof data to a file
/// This exports the complete proof data needed for the confidential_transfer_transfer_with_fee instruction
pub fn export_transfer_with_proof(
    proof_data: &TransferWithFeeInstructionData,
    amount: u64,
    source_pubkey: Option<String>,
    destination_pubkey: Option<String>,
    path: &Path,
) -> Result<()> {
    // Convert to DTO and serialize
    let dto = TransferWithFeeDto::from(proof_data);
    let serialized_proof = serde_json::to_string(&dto)?;
    
    // Create the exportable proof
    let exportable_proof = ExportableProof {
        proof_type: ProofType::TransferWithProof,
        data: general_purpose::STANDARD.encode(serialized_proof),
        metadata: ProofMetadata {
            source_address: source_pubkey,
            destination_address: destination_pubkey,
            mint_address: None, // This should be populated in the new implementation
            amount: Some(amount),
            timestamp: Utc::now().to_rfc3339(),
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
    _proof_data: &WithdrawProofData,
    amount: u64,
    source_pubkey: Option<String>,
    path: &Path,
) -> Result<()> {
    // Since WithdrawProofData doesn't implement Serialize, we'll create a custom serializable wrapper
    let serializable_proof = SerializableProofData {
        data_type: "withdraw_proof".to_string(),
        // In a real implementation we would serialize the proof data properly
        binary_data: vec![0; 64], // Placeholder
    };
    
    // Serialize our wrapper instead
    let serialized_proof = serde_json::to_string(&serializable_proof)?;
    
    // Create the exportable proof
    let exportable_proof = ExportableProof {
        proof_type: ProofType::Withdraw,
        data: general_purpose::STANDARD.encode(serialized_proof),
        metadata: ProofMetadata {
            source_address: source_pubkey,
            destination_address: None,
            mint_address: None, // This should be populated in the new implementation
            amount: Some(amount),
            timestamp: Utc::now().to_rfc3339(),
        },
    };
    
    // Write to file
    let serialized = serde_json::to_string_pretty(&exportable_proof)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    
    Ok(())
}

/// Export a withdraw with proof data to a file
/// This exports the complete proof data needed for the confidential_transfer_withdraw instruction
pub fn export_withdraw_with_proof(
    proof_data: &WithdrawInstructionData,
    amount: u64,
    source_pubkey: Option<String>,
    path: &Path,
) -> Result<()> {
    // Convert to DTO and serialize
    let dto = WithdrawDto::from(proof_data);
    let serialized_proof = serde_json::to_string(&dto)?;
    
    // Create the exportable proof
    let exportable_proof = ExportableProof {
        proof_type: ProofType::WithdrawWithProof,
        data: general_purpose::STANDARD.encode(serialized_proof),
        metadata: ProofMetadata {
            source_address: source_pubkey,
            destination_address: None,
            mint_address: None, // This should be populated in the new implementation
            amount: Some(amount),
            timestamp: Utc::now().to_rfc3339(),
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
    
    // Create a placeholder TransferProofData
    let proof_data: TransferProofData = unsafe { std::mem::zeroed() };
    
    Ok(proof_data)
}

/// Extract a transfer with proof data from an imported exportable proof
pub fn extract_transfer_with_proof(proof: &ExportableProof) -> Result<TransferWithFeeInstructionData> {
    if proof.proof_type != ProofType::TransferWithProof {
        return Err(anyhow!("Invalid proof type: expected TransferWithProof"));
    }
    
    // Decode base64 and deserialize from JSON to DTO
    let json_string = String::from_utf8(general_purpose::STANDARD.decode(&proof.data)?)?;
    let dto: TransferWithFeeDto = serde_json::from_str(&json_string)?;
    
    // Convert DTO to instruction data
    Ok(dto.to_instruction_data())
}

/// Extract a withdraw proof from an imported exportable proof
pub fn extract_withdraw_proof(proof: &ExportableProof) -> Result<WithdrawProofData> {
    if proof.proof_type != ProofType::Withdraw {
        return Err(anyhow!("Invalid proof type: expected Withdraw proof"));
    }
    
    // Create a placeholder WithdrawProofData
    let proof_data: WithdrawProofData = unsafe { std::mem::zeroed() };
    
    Ok(proof_data)
}

/// Extract a withdraw with proof data from an imported exportable proof
pub fn extract_withdraw_with_proof(proof: &ExportableProof) -> Result<WithdrawInstructionData> {
    if proof.proof_type != ProofType::WithdrawWithProof {
        return Err(anyhow!("Invalid proof type: expected WithdrawWithProof"));
    }
    
    // Decode base64 and deserialize from JSON to DTO
    let json_string = String::from_utf8(general_purpose::STANDARD.decode(&proof.data)?)?;
    let dto: WithdrawDto = serde_json::from_str(&json_string)?;
    
    // Convert DTO to instruction data
    Ok(dto.to_instruction_data())
}

/// Create a demo transfer proof and export it to a file
pub fn create_and_export_demo_transfer_proof(
    amount: u64,
    source_keypair: &ElGamalKeypair,
    destination_pubkey: &ElGamalPubkey,
    path: &Path,
) -> Result<()> {
    let proof_data = zk_proofs::generate_transfer_proof(
        amount,
        source_keypair,
        destination_pubkey,
        None,
    )?;
    
    export_transfer_proof(
        &proof_data,
        amount,
        Some(bs58::encode(&Into::<[u8; 32]>::into(source_keypair.pubkey())).into_string()),
        Some(bs58::encode(&Into::<[u8; 32]>::into(destination_pubkey.clone())).into_string()),
        path,
    )
}

/// Create a demo transfer with proof and export it to a file
pub fn create_and_export_demo_transfer_with_proof(
    amount: u64,
    source_keypair: &ElGamalKeypair,
    source_available_balance: &[u8; 64],
    destination_pubkey: &ElGamalPubkey,
    path: &Path,
) -> Result<()> {
    let proof_data = zk_proofs::generate_transfer_with_proof_data(
        amount,
        source_keypair,
        source_available_balance,
        destination_pubkey,
        None,
    )?;
    
    export_transfer_with_proof(
        &proof_data,
        amount,
        Some(bs58::encode(&Into::<[u8; 32]>::into(source_keypair.pubkey())).into_string()),
        Some(bs58::encode(&Into::<[u8; 32]>::into(destination_pubkey.clone())).into_string()),
        path,
    )
}

/// Create a demo withdraw proof and export it to a file
pub fn create_and_export_demo_withdraw_proof(
    amount: u64,
    keypair: &ElGamalKeypair,
    path: &Path,
) -> Result<()> {
    let proof_data = zk_proofs::generate_withdraw_proof(
        amount,
        keypair,
        None,
    )?;
    
    export_withdraw_proof(
        &proof_data,
        amount,
        Some(bs58::encode(&Into::<[u8; 32]>::into(keypair.pubkey())).into_string()),
        path,
    )
}

/// Create a demo withdraw with proof and export it to a file
pub fn create_and_export_demo_withdraw_with_proof(
    amount: u64,
    keypair: &ElGamalKeypair,
    available_balance: &[u8; 64],
    path: &Path,
) -> Result<()> {
    let proof_data = zk_proofs::generate_withdraw_with_proof_data(
        amount,
        keypair,
        available_balance,
        None,
    )?;
    
    export_withdraw_with_proof(
        &proof_data,
        amount,
        Some(bs58::encode(&Into::<[u8; 32]>::into(keypair.pubkey())).into_string()),
        path,
    )
}

/// Export a transfer proof to a file with mint address in metadata
pub fn export_transfer_proof_with_mint(
    proof_data: &TransferProofData,
    amount: u64,
    source_pubkey: Option<String>,
    destination_pubkey: Option<String>,
    mint_address: Option<String>,
    path: &Path,
) -> Result<()> {
    // Since TransferProofData doesn't implement Serialize, we'll create a custom serializable wrapper
    let serializable_proof = SerializableProofData {
        data_type: "transfer_proof".to_string(),
        // In a real implementation we would serialize the proof data properly
        binary_data: vec![0; 64], // Placeholder
    };
    
    // Serialize our wrapper instead
    let serialized_proof = serde_json::to_string(&serializable_proof)?;
    
    // Create a unique proof ID
    let proof_id = Uuid::new_v4().to_string();
    
    // Create the exportable proof
    let exportable_proof = ExportableProof {
        version: "1.0.0".to_string(),
        proof_id,
        proof_type: ProofType::Transfer,
        zk_sdk_version: get_zk_sdk_version(),
        data: general_purpose::STANDARD.encode(serialized_proof),
        metadata: ProofMetadata {
            source_address: source_pubkey,
            destination_address: destination_pubkey,
            mint_address,
            amount: Some(amount),
            timestamp: Utc::now().to_rfc3339(),
        },
    };
    
    // Write to file
    let serialized = serde_json::to_string_pretty(&exportable_proof)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    
    Ok(())
}

/// Export a withdraw proof to a file with mint address in metadata
pub fn export_withdraw_proof_with_mint(
    proof_data: &WithdrawProofData,
    amount: u64,
    source_pubkey: Option<String>,
    mint_address: Option<String>,
    path: &Path,
) -> Result<()> {
    // Since WithdrawProofData doesn't implement Serialize, we'll create a custom serializable wrapper
    let serializable_proof = SerializableProofData {
        data_type: "withdraw_proof".to_string(),
        // In a real implementation we would serialize the proof data properly
        binary_data: vec![0; 64], // Placeholder
    };
    
    // Serialize our wrapper instead
    let serialized_proof = serde_json::to_string(&serializable_proof)?;
    
    // Create a unique proof ID
    let proof_id = Uuid::new_v4().to_string();
    
    // Create the exportable proof
    let exportable_proof = ExportableProof {
        version: "1.0.0".to_string(),
        proof_id,
        proof_type: ProofType::Withdraw,
        zk_sdk_version: get_zk_sdk_version(),
        data: general_purpose::STANDARD.encode(serialized_proof),
        metadata: ProofMetadata {
            source_address: source_pubkey,
            destination_address: None,
            mint_address,
            amount: Some(amount),
            timestamp: Utc::now().to_rfc3339(),
        },
    };
    
    // Write to file
    let serialized = serde_json::to_string_pretty(&exportable_proof)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    
    Ok(())
} 