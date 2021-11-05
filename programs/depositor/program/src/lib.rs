#![deny(missing_docs)]

//! Depositor contract

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("A3oFe74to7813qF5KhjTJeR4eTFZYKduXipV62tVZouC");

/// Generates seed bump for authorities
pub fn find_program_address(program_id: &Pubkey, pubkey: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&pubkey.to_bytes()[..32]], program_id)
}

/// Generates transit address
pub fn find_transit_program_address(
    program_id: &Pubkey,
    depositor: &Pubkey,
    token_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&depositor.to_bytes(), &token_mint.to_bytes()], program_id)
}
