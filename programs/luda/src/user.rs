use solana_program::pubkey::Pubkey;
use crate::DLU_wallet::DLUWallet;
use solana_program::borsh::{BorshSerialize, BorshDeserialize};


/// Represents the status of a user based on their performance in deals and shipments.
pub enum UserStatus {
    New,
    Credible,
    Reliable,
    Risky,
    Unreliable,
    Suspicious,
    Fraud,
}

/// Represents a user in the system, tracking their details, wallet, and performance metrics.
pub struct User {
    pub username: String,
    pub pubkey: Pubkey,
    pub wallet: DLUWallet,
    pub status: UserStatus,
    pub total_deals: u32,
    pub successful_deals: u32,
    pub failed_deals: u32,
    pub total_shipments: u32,
    pub successful_shipments: u32,
    pub failed_shipments: u32,
}

impl User {
    /// Creates a new user with initial values.
    pub fn new(username: String, pubkey: Pubkey, wallet: DLUWallet) -> Self {
        User {
            username,
            pubkey,
            wallet,
            status: UserStatus::New,
            total_deals: 0,
            successful_deals: 0,
            failed_deals: 0,
            total_shipments: 0,
            successful_shipments: 0,
            failed_shipments: 0,
        }
    }

    /// Increments the deal counters based on the outcome.
    pub fn mark_deal(&mut self, successful: bool) {
        self.total_deals += 1;
        if successful {
            self.successful_deals += 1;
        } else {
            self.failed_deals += 1;
        }
        self.update_status();
    }

    /// Increments the shipment counters based on the outcome.
    pub fn mark_shipment(&mut self, successful: bool) {
        self.total_shipments += 1;
        if successful {
            self.successful_shipments += 1;
        } else {
            self.failed_shipments += 1;
        }
        self.update_status();
    }

    /// Updates the status of a user based on the success rate of their deals and shipments.
    pub fn update_status(&mut self) {
        let total_operations = self.total_deals + self.total_shipments;
        let successful_operations = self.successful_deals + self.successful_shipments;

        if total_operations < 3 {
            self.status = UserStatus::New;
        } else if total_operations > 10 && successful_operations == total_operations {
            self.status = UserStatus::Credible;
        } else {
            let success_rate = successful_operations as f32 / total_operations as f32;
            self.status = if success_rate >= 0.9 {
                UserStatus::Reliable
            } else if success_rate >= 0.7 {
                UserStatus::Risky
            } else if success_rate >= 0.5 {
                UserStatus::Unreliable
            } else if success_rate >= 0.2 {
                UserStatus::Suspicious
            } else {
                UserStatus::Fraud
            };
        }
    }

    /// Serializes the user into a vector of bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        self.try_to_vec().map_err(|_| "Failed to serialize User")
    }

    /// Deserializes a user from a slice of bytes.
    pub fn deserialize(input: &mut &[u8]) -> Result<Self, &'static str> {
        Self::try_from_slice(input).map_err(|_| "Failed to deserialize User")
    }
}
