use anyhow::{anyhow, Result};
use semver::Version;
use std::cmp::Ordering;
use crate::proof_export::ExportableProof;

/// Current version of the DTO format
pub const CURRENT_DTO_VERSION: &str = "1.0.0";

/// Minimum version of the DTO format that we support
pub const MIN_SUPPORTED_DTO_VERSION: &str = "1.0.0";

/// Check if a proof DTO's version is compatible with the current implementation
pub fn check_version_compatibility(proof: &ExportableProof) -> Result<bool> {
    // Parse the version strings
    let current_version = Version::parse(CURRENT_DTO_VERSION)
        .map_err(|e| anyhow!("Failed to parse current version: {}", e))?;
    let min_supported_version = Version::parse(MIN_SUPPORTED_DTO_VERSION)
        .map_err(|e| anyhow!("Failed to parse minimum supported version: {}", e))?;
    let proof_version = Version::parse(&proof.version)
        .map_err(|e| anyhow!("Failed to parse proof version: {}", e))?;
    
    // Check if the proof version is compatible
    // It's compatible if it's greater than or equal to the minimum supported version
    // and less than or equal to the current version
    let is_compatible = proof_version >= min_supported_version && proof_version <= current_version;
    
    Ok(is_compatible)
}

/// Check if a proof needs to be upgraded to the current version
pub fn needs_upgrade(proof: &ExportableProof) -> Result<bool> {
    // Parse the version strings
    let current_version = Version::parse(CURRENT_DTO_VERSION)
        .map_err(|e| anyhow!("Failed to parse current version: {}", e))?;
    let proof_version = Version::parse(&proof.version)
        .map_err(|e| anyhow!("Failed to parse proof version: {}", e))?;
    
    // Check if the proof version is less than the current version
    Ok(proof_version < current_version)
}

/// Get the version relationship between a proof and the current version
pub fn get_version_relationship(proof: &ExportableProof) -> Result<Ordering> {
    // Parse the version strings
    let current_version = Version::parse(CURRENT_DTO_VERSION)
        .map_err(|e| anyhow!("Failed to parse current version: {}", e))?;
    let proof_version = Version::parse(&proof.version)
        .map_err(|e| anyhow!("Failed to parse proof version: {}", e))?;
    
    // Compare the versions
    Ok(proof_version.cmp(&current_version))
}

/// Upgrade a proof to the current version if needed
pub fn upgrade_proof_to_current_version(proof: &mut ExportableProof) -> Result<bool> {
    if !needs_upgrade(proof)? {
        // No upgrade needed
        return Ok(false);
    }
    
    // Get the version relationship
    let relationship = get_version_relationship(proof)?;
    
    match relationship {
        Ordering::Less => {
            // Proof version is less than current version
            // This is where we'd implement version-specific upgrades
            // For now, we just update the version field
            proof.version = CURRENT_DTO_VERSION.to_string();
            
            // If we have older versions in the future, we would add logic like:
            // if proof.version == "0.9.0" {
            //    // Upgrade from 0.9.0 to 1.0.0
            //    // ... upgrade logic
            // }
            
            Ok(true)
        },
        _ => {
            // No upgrade needed or not possible
            Ok(false)
        }
    }
} 