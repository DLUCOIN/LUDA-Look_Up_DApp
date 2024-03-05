use solana_program::{
    clock::Clock,
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use crate::dlu_token::DLUToken;

// Define the PENALTY_ACCOUNT pubkey
const PENALTY_ACCOUNT: Pubkey = Pubkey::new_from_array([your_penalty_account_bytes_here]);

pub struct Escrow;

impl Escrow {
    pub fn lock_funds(
        user_account: &AccountInfo,
        escrow_account: &AccountInfo,
        authority_info: &AccountInfo,
        amount: u64,
    ) -> Result<(), ProgramError> {
        DLUToken::transfer(user_account, escrow_account, authority_info, amount)
    }

    pub fn release_funds(
        escrow_account: &AccountInfo,
        recipient_account: &AccountInfo,
        escrow_authority_info: &AccountInfo,
        amount: u64,
    ) -> Result<(), ProgramError> {
        DLUToken::transfer(escrow_account, recipient_account, escrow_authority_info, amount)
    }

    pub fn transfer_to_penalty(
        escrow_account: &AccountInfo,
        escrow_authority_info: &AccountInfo,
        amount: u64,
    ) -> Result<(), ProgramError> {
        // Transfer funds from escrow account to penalty account
        DLUToken::transfer(escrow_account, &PENALTY_ACCOUNT, escrow_authority_info, amount)
    }

    pub fn handle_smart_deal(
        seller: &AccountInfo,
        buyer: &AccountInfo,
        seller_key: Option<String>,
        buyer_key: Option<String>,
        insurance: u64,
        price: u64,
    ) -> Result<(), ProgramError> {
        match (seller_key, buyer_key) {
            (Some(s), Some(b)) => {
                Self::release_funds(seller, price)?;
                Self::release_funds(seller, insurance)?;
                Self::release_funds(buyer, insurance)?;
            }
            (Some(s), None) => {
                let total_amount = price + 2 * insurance;
                Self::transfer_to_penalty(seller, total_amount)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_smart_shipment(
        sender: &AccountInfo,
        carrier: &AccountInfo,
        sender_key: Option<String>,
        carrier_key: Option<String>,
        recipient_key: Option<String>,
        payment: u64,
        insurance: u64,
    ) -> Result<(), ProgramError> {
        match (sender_key, carrier_key, recipient_key) {
            (None, Some(c), Some(r)) => {
                let total_amount = payment + insurance;
                Self::release_funds(carrier, total_amount)?;
            }
            (Some(s), Some(c), None) => {
                let total_amount = payment + insurance;
                Self::transfer_to_penalty(sender, total_amount)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_expired_deal(
        seller: &AccountInfo,
        buyer: &AccountInfo,
        insurance: u64,
        price: u64,
        meeting_datetime: i64,
    ) -> Result<(), ProgramError> {
        let clock = Clock::get()?;
        if clock.unix_timestamp > meeting_datetime + 24 * 60 * 60 {
            Self::release_funds(seller, insurance)?;
            Self::release_funds(buyer, insurance + price)?;
        }
        Ok(())
    }

    pub fn handle_expired_shipment(
        sender: &AccountInfo,
        carrier: &AccountInfo,
        payment: u64,
        insurance: u64,
        drop_off_datetime: i64,
    ) -> Result<(), ProgramError> {
        let clock = Clock::get()?;
        if clock.unix_timestamp > drop_off_datetime + 24 * 60 * 60 {
            Self::release_funds(sender, payment)?;
            Self::release_funds(carrier, insurance)?;
        }
        Ok(())
    }
    
    pub fn cancel_shipment(
        &mut self,
        sender_account: &AccountInfo,
        escrow_account: &AccountInfo,
        escrow_authority_info: &AccountInfo,
    ) -> Result<(), &'static str> {
        // Ensure the shipment is in the 'Listed' state.
        if self.status != ShipmentStatus::Listed {
            return Err("Shipment is not in the 'Listed' state or has already been accepted.");
        }

        // Release the locked payment back to the sender's account.
        Self::release_funds(escrow_account, sender_account, escrow_authority_info, self.payment)?;

        // Invalidate the sender's key.
        self.sender_key.clear();

        // Update the status of the shipment to 'Canceled'.
        self.status = ShipmentStatus::Canceled;

        Ok(())
    }
	
}
