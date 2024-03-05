use anchor_lang::prelude::*;
use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};
use crate::processor::Processor;

declare_id!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");

#[program]
pub mod luda {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

pub mod user;         // User profiles, status, etc.
pub mod offer;        // Offers posted by sellers
pub mod request;      // Requests posted by buyers
pub mod shipment;     // Shipment details and tracking
pub mod dlu_token;    // DLU token related operations
pub mod dlu_wallet;   // DLU wallet operations
pub mod escrow;       // Escrow operations
pub mod onetimekeys;  // Generation and management of one-time keys
pub mod addressing;   // Entities addressing
pub mod processor;    // Core processing logic
pub mod error;        // Error handling

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    Processor::process(program_id, accounts, input)
}

