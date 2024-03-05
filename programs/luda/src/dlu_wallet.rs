use solana_program::pubkey::Pubkey;
use crate::dlu_token::DLUToken;
use crate::escrow::Escrow;

pub struct Wallet {
    pub owner: Pubkey,  // Owner of the DLU wallet.
    pub balance: u64,   // The current DLU balance.
}

impl Wallet {
    /// Initializes a new Wallet.
    pub fn new(owner: Pubkey) -> Self {
        Wallet {
            owner,
            balance: 0,  // Initial balance is 0.
        }
    }

    /// Fetches the latest balance from the DLU token ledger.
    pub fn refresh_balance(&mut self) {
        // Fetch the balance from DLUtoken.rs (This is a mock, in real-world it would query the ledger.)
        self.balance = DLUToken::get_balance(&self.owner).unwrap_or(0);
    }

    /// Deducts a specified amount from the wallet.
    pub fn deduct(&mut self, amount: u64) -> Result<(), &'static str> {
        if self.balance < amount {
            return Err("Insufficient funds in wallet.");
        }
        self.balance -= amount;  // Deduct the specified amount from the wallet's balance.
        Ok(())
    }

    /// Locks a specified amount in escrow.
    pub fn lock_for_escrow(&mut self, amount: u64) -> Result<u64, &'static str> {
        // Lock the specified amount in escrow and get the escrow ID.
        let escrow_id = Escrow::lock_funds(&self.owner, amount)?;
        Ok(escrow_id)
    }

    /// Releases a previously locked amount from escrow back to the wallet.
    pub fn release_from_escrow(&mut self, amount: u64, escrow_id: u64) -> Result<(), &'static str> {
        // Call to DLUtoken.rs to release the funds from the escrow back to the wallet using the escrow ID.
        DLUToken::transfer_from_escrow(escrow_id, &self.owner, amount)?;
        self.refresh_balance(); // Refresh balance after the operation.
        Ok(())
    }

    /// Transfers DLU from this wallet to another.
    pub fn transfer(&mut self, recipient: &mut Wallet, amount: u64) -> Result<(), &'static str> {
        if self.balance < amount {
            return Err("Insufficient funds");
        }
        // Call to DLUtoken.rs to perform the transfer.
        DLUToken::transfer(&self.owner, &recipient.owner, amount)?;
        self.refresh_balance(); // Refresh balance after the operation.
        recipient.refresh_balance();
        Ok(())
    }
}
