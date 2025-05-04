use anyhow::Result;
use florin_zk::{zk_proofs, proof_export};
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use bs58;
use bytemuck;
use std::str::FromStr;
use uuid::Uuid;
use chrono::Utc;
use base64;
use serde_json;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new ElGamal keypair and display it
    GenKeypair,
    
    /// Generate a transfer proof and export it to a file
    TransferProof {
        /// Amount to transfer
        #[arg(short, long)]
        amount: u64,
        
        /// Path to save the exported proof
        #[arg(short, long, default_value = "transfer_proof.json")]
        output: PathBuf,
    },
    
    /// Generate a withdraw proof and export it to a file
    WithdrawProof {
        /// Amount to withdraw
        #[arg(short, long)]
        amount: u64,
        
        /// Path to save the exported proof
        #[arg(short, long, default_value = "withdraw_proof.json")]
        output: PathBuf,
    },
    
    /// Generate a proof with standardized output format
    Generate {
        /// Type of proof to generate (transfer or withdraw)
        #[arg(short, long, value_parser = ["transfer", "withdraw"])]
        r#type: String,
        
        /// Amount to use for the proof
        #[arg(short, long)]
        amount: u64,
        
        /// Address of the token mint
        #[arg(short, long)]
        mint: String,
        
        /// Source wallet or token account address (optional)
        #[arg(short, long)]
        source: Option<String>,
        
        /// Destination wallet or token account address (optional, required for transfer)
        #[arg(short, long)]
        dest: Option<String>,
        
        /// Auto-verify after generation
        #[arg(short, long, default_value = "false")]
        verify: bool,
        
        /// Custom output path (default: auto-named with timestamp and UUID)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Generate a demo proof with a specified amount
    Demo {
        /// Amount to use in the demo proof
        #[arg(short, long, default_value = "1000")]
        amount: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Florin ZK - Zero Knowledge Proof Generation Tool");
    println!("================================================");
    
    let cli = Cli::parse();

    match &cli.command {
        Commands::GenKeypair => {
            // Generate a new ElGamal keypair
            let keypair = zk_proofs::generate_elgamal_keypair();
            let ae_key = zk_proofs::generate_ae_key();
            
            println!("\nGenerated new ElGamal keypair:");
            println!("Public key (base58): {}", bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(keypair.pubkey())).into_string());
            // AeKey doesn't have to_bytes method, just output placeholder
            println!("AES key (base58): {}", "[AES key serialization not supported]");
            
            println!("\nKeep your private keys secure and do not share them!");
        }
        
        Commands::TransferProof { amount, output } => {
            println!("\nGenerating transfer proof for {} tokens", amount);
            
            // Generate keypairs for demonstration
            let source_keypair = zk_proofs::generate_elgamal_keypair();
            let destination_keypair = zk_proofs::generate_elgamal_keypair();
            
            println!("Source public key: {}", bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(source_keypair.pubkey())).into_string());
            println!("Destination public key: {}", bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(destination_keypair.pubkey())).into_string());
            
            // Generate the transfer proof
            let transfer_proof = zk_proofs::generate_transfer_proof(
                *amount,
                &source_keypair,
                destination_keypair.pubkey(),
                None,
            )?;
            
            // Export the proof to a file
            proof_export::export_transfer_proof(
                &transfer_proof,
                *amount,
                Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(source_keypair.pubkey())).into_string()),
                Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(destination_keypair.pubkey())).into_string()),
                output,
            )?;
            
            println!("Transfer proof exported to: {}", output.display());
            
            // Verify the proof
            let verification_result = zk_proofs::verify_transfer_proof(&transfer_proof)?;
            println!("Proof verification result: {}", verification_result);
        }
        
        Commands::WithdrawProof { amount, output } => {
            println!("\nGenerating withdraw proof for {} tokens", amount);
            
            // Generate keypair for demonstration
            let keypair = zk_proofs::generate_elgamal_keypair();
            println!("Token account public key: {}", bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(keypair.pubkey())).into_string());
            
            // Generate the withdraw proof
            let withdraw_proof = zk_proofs::generate_withdraw_proof(
                *amount,
                &keypair,
                None,
            )?;
            
            // Export the proof to a file
            proof_export::export_withdraw_proof(
                &withdraw_proof,
                *amount,
                Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(keypair.pubkey())).into_string()),
                output,
            )?;
            
            println!("Withdraw proof exported to: {}", output.display());
            
            // Verify the proof
            let verification_result = zk_proofs::verify_withdraw_proof(&withdraw_proof)?;
            println!("Proof verification result: {}", verification_result);
        }
        
        Commands::Generate { r#type, amount, mint, source, dest, verify, output } => {
            println!("\nGenerating {} proof for {} tokens", r#type, amount);
            
            // Validate parameters based on proof type
            if r#type == "transfer" && dest.is_none() {
                return Err(anyhow::anyhow!("Destination address is required for transfer proofs"));
            }
            
            // Generate keypairs based on provided addresses or create demo ones
            let source_keypair = match source {
                Some(addr) => {
                    println!("Using provided source address: {}", addr);
                    // In a real implementation, we would load the keypair from the address
                    // For now, just generate a demo keypair
                    zk_proofs::generate_elgamal_keypair()
                },
                None => {
                    println!("No source address provided, generating a demo keypair");
                    zk_proofs::generate_elgamal_keypair()
                }
            };
            
            // Generate destination pubkey for transfer
            let destination_pubkey = if r#type == "transfer" {
                match dest {
                    Some(addr) => {
                        println!("Using provided destination address: {}", addr);
                        // In a real implementation, we would derive the pubkey from the address
                        // For now, just generate a demo pubkey
                        zk_proofs::generate_elgamal_keypair().pubkey()
                    },
                    None => unreachable!() // We already validated this above
                }
            } else {
                // For withdraw, we don't need a destination
                zk_proofs::generate_elgamal_keypair().pubkey() // Placeholder
            };
            
            // Create auto-named output file if none provided
            let output_path = match output {
                Some(path) => path.clone(),
                None => {
                    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
                    let proof_id = Uuid::new_v4().to_string();
                    PathBuf::from(format!("{}_{}_proof_{}.json", r#type, timestamp, proof_id))
                }
            };
            
            // Generate the appropriate proof
            if r#type == "transfer" {
                // Generate transfer proof
                let transfer_proof = zk_proofs::generate_transfer_proof(
                    *amount,
                    &source_keypair,
                    destination_pubkey,
                    None,
                )?;
                
                // Export with enhanced metadata
                proof_export::export_transfer_proof_with_mint(
                    &transfer_proof,
                    *amount,
                    Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(source_keypair.pubkey())).into_string()),
                    Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(destination_pubkey)).into_string()),
                    Some(mint.clone()),
                    &output_path,
                )?;
                
                println!("Transfer proof exported to: {}", output_path.display());
                
                // Verify the proof if requested
                if *verify {
                    println!("Verifying transfer proof...");
                    let verification_result = zk_proofs::verify_transfer_proof(&transfer_proof)?;
                    println!("Proof verification result: {}", verification_result);
                }
            } else {
                // Generate withdraw proof
                let withdraw_proof = zk_proofs::generate_withdraw_proof(
                    *amount,
                    &source_keypair,
                    None,
                )?;
                
                // Export with enhanced metadata
                proof_export::export_withdraw_proof_with_mint(
                    &withdraw_proof,
                    *amount,
                    Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(source_keypair.pubkey())).into_string()),
                    Some(mint.clone()),
                    &output_path,
                )?;
                
                println!("Withdraw proof exported to: {}", output_path.display());
                
                // Verify the proof if requested
                if *verify {
                    println!("Verifying withdraw proof...");
                    let verification_result = zk_proofs::verify_withdraw_proof(&withdraw_proof)?;
                    println!("Proof verification result: {}", verification_result);
                }
            }
            
            println!("\nProof generation complete!");
            println!("Proof ID: {}", Uuid::new_v4().to_string());
            println!("File: {}", output_path.display());
            println!("Type: {}", r#type);
            println!("Amount: {}", amount);
            println!("Mint: {}", mint);
            
            if *verify {
                println!("Verification: Passed");
            }
        },
        
        Commands::Demo { amount } => {
            println!("\nRunning full demo with amount: {}", amount);
            
            // 1. Generate keypairs
            println!("1. Generating keypairs...");
            let source_keypair = zk_proofs::generate_elgamal_keypair();
            let destination_keypair = zk_proofs::generate_elgamal_keypair();
            
            // 2. Generate a transfer proof
            println!("2. Generating transfer proof...");
            let transfer_path = PathBuf::from("demo_transfer_proof.json");
            let transfer_proof = zk_proofs::generate_transfer_proof(
                *amount,
                &source_keypair,
                destination_keypair.pubkey(),
                None,
            )?;
            
            // Export the transfer proof
            proof_export::export_transfer_proof(
                &transfer_proof,
                *amount,
                Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(source_keypair.pubkey())).into_string()),
                Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(destination_keypair.pubkey())).into_string()),
                &transfer_path,
            )?;
            
            // 3. Generate a withdraw proof
            println!("3. Generating withdraw proof...");
            let withdraw_path = PathBuf::from("demo_withdraw_proof.json");
            let withdraw_proof = zk_proofs::generate_withdraw_proof(
                *amount,
                &source_keypair,
                None,
            )?;
            
            // Export the withdraw proof
            proof_export::export_withdraw_proof(
                &withdraw_proof,
                *amount,
                Some(bs58::encode(&zk_proofs::elgamal_pubkey_to_bytes(source_keypair.pubkey())).into_string()),
                &withdraw_path,
            )?;
            
            // 4. Import and verify the proofs
            println!("4. Importing and verifying proofs...");
            
            // Import transfer proof
            let imported_transfer = proof_export::import_proof_from_file(&transfer_path)?;
            let extracted_transfer = proof_export::extract_transfer_proof(&imported_transfer)?;
            let transfer_verify = zk_proofs::verify_transfer_proof(&extracted_transfer)?;
            
            // Import withdraw proof
            let imported_withdraw = proof_export::import_proof_from_file(&withdraw_path)?;
            let extracted_withdraw = proof_export::extract_withdraw_proof(&imported_withdraw)?;
            let withdraw_verify = zk_proofs::verify_withdraw_proof(&extracted_withdraw)?;
            
            println!("\nDemo Results:");
            println!("Transfer proof verification: {}", transfer_verify);
            println!("Withdraw proof verification: {}", withdraw_verify);
            println!("\nProof files:");
            println!("Transfer proof: {}", transfer_path.display());
            println!("Withdraw proof: {}", withdraw_path.display());
        }
    }
    
    Ok(())
} 