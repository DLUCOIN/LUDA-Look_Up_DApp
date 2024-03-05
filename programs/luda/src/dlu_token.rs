use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    program::{invoke},
};
use spl_token::{self, state::Account as TokenAccount};

pub struct DLUToken;

impl DLUToken {
    // Check the DLU balance of a specific account.
    pub fn get_balance(account_info: &AccountInfo) -> Result<u64, ProgramError> {
        let token_account_data = TokenAccount::unpack(&account_info.data.borrow())?;
        Ok(token_account_data.amount)
    }

    // Transfers DLU tokens from one account to another.
    pub fn transfer(
        src_account_info: &AccountInfo,
        dest_account_info: &AccountInfo,
        authority_info: &AccountInfo,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let transfer_instruction = spl_token::instruction::transfer(
            &spl_token::id(),
            &src_account_info.key,
            &dest_account_info.key,
            &authority_info.key,
            &[],
            amount,
        )?;
        
        invoke(
            &transfer_instruction, 
            &[src_account_info.clone(), dest_account_info.clone(), authority_info.clone()]
        )
    }

    // Checks if the provided authority can move DLU from the specified account.
    pub fn check_authority(
        token_account_info: &AccountInfo,
        authority_pubkey: &Pubkey,
    ) -> Result<bool, ProgramError> {
        let token_account_data = TokenAccount::unpack(&token_account_info.data.borrow())?;
        Ok(token_account_data.owner == *authority_pubkey)
    }
    
}
