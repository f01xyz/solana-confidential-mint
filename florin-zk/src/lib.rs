// Re-export modules from lib directory
pub mod lib {
    pub mod zk_proofs;
    pub mod proof_export;
}

// Re-export at the crate root for convenient access
pub use lib::zk_proofs;
pub use lib::proof_export; 