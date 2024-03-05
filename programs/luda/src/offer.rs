use crate::user::User; 
use crate::onetimekeys::Onetimekeys;
use crate::dlu_wallet::Wallet;
use crate::escrow::Escrow; 
use chrono::{DateTime, Utc}; 
use solana_program::borsh::{BorshSerialize, BorshDeserialize};


/// Represents an in-game location.
pub struct Location {
    country: String,
    town: String,
    address: String,
}

/// Represents the current status of an offer.
pub enum OfferStatus {
    Listed,
    Accepted,
    Completed,
    Failed,
    Expired,
    Canceled,
}

/// Represents a single offer posted by a seller.
pub struct Offer {
    id: u64,
    status: OfferStatus,
    seller: User,
    buyer: Option<User>,
    meeting_point: Location,
    meeting_datetime: DateTime<Utc>,
    payment: u64,
    insurance: u64,
    goodsorservice_name: String,
    goodsorservice_description: String,
    seller_key: String,
    buyer_key: String,
    escrow_id: u64,
}

impl Offer {
    /// List a new offer.
    pub fn list_offer(
        id: u64,
        seller: &mut User,
        goodsorservice_name: String,
        goodsorservice_description: String,
        payment: u64,
        meeting_point: Location,
        meeting_datetime: DateTime<Utc>,
    ) -> Result<Self, &'static str> {
        // Insurance is always equal to payment.
        let insurance = payment;

        // Check seller's balance for sufficient funds for insurance.
        if seller.wallet.balance < insurance {
            return Err("Insufficient funds for insurance.");
        }

        // Deduct insurance amount from seller's wallet.
        seller.wallet.balance -= insurance;

        // Lock insurance amount in escrow.
        let escrow_id = Escrow::lock_funds(&seller.wallet, insurance)?;

        Ok(Offer {
            id,
            status: OfferStatus::Listed,
            seller: seller.clone(),
            buyer: None,
            meeting_point,
            meeting_datetime,
            payment,
            insurance,
            goodsorservice_name,
            goodsorservice_description,
            seller_key: String::new(),
            buyer_key: String::new(),
            escrow_id,
        })
    }

    pub fn accept_offer(
        &mut self, 
        buyer: &mut User,
        buyer_account: &AccountInfo, 
        escrow_account: &AccountInfo, 
        authority_info: &AccountInfo
    ) -> Result<(), &'static str> {
        // Ensure the offer is in the 'Listed' state.
        if self.status != OfferStatus::Listed {
            return Err("Offer is not in the 'Listed' state.");
        }
        
        // Generate the one-time keys for both seller and buyer.
        self.seller_key = onetimekeys::generate_key(); 
        self.buyer_key = onetimekeys::generate_key();

        // Update the buyer field.
        self.buyer = Some(buyer.clone());

        // Check buyer's balance.
        let buyer_balance = DLUToken::get_balance(buyer_account)?;
        let total_deduction = self.payment + self.insurance;
        if buyer_balance < total_deduction {
            return Err("Insufficient funds in buyer's account.");
        }

        // Deduct the payment and insurance amounts from the buyer's wallet.
        buyer.wallet.balance -= total_deduction;

        // Lock the payment and insurance amounts in escrow.
        Escrow::lock_funds(buyer_account, escrow_account, authority_info, total_deduction)?;

        // Update the status of the offer to 'Accepted'.
        self.status = OfferStatus::Accepted;

        Ok(())
    }

    pub fn complete_offer(
        &mut self, 
        entered_buyer_key: String, 
        entered_seller_key: String,
        seller_account: &AccountInfo,
        buyer_account: &AccountInfo,
        escrow_account: &AccountInfo,
        escrow_authority_info: &AccountInfo,
        seller: &mut User,
        buyer: &mut User,
    ) -> Result<(), &'static str> {
        // Ensure the offer is in the 'Accepted' state.
        if self.status != OfferStatus::Accepted {
            return Err("Offer is not in the 'Accepted' state.");
        }

        // Validate the buyer's key.
        if entered_buyer_key != self.buyer_key {
            return Err("Invalid buyer key provided.");
        }

        // Check escrow balance.
        let escrow_balance = DLUToken::get_balance(escrow_account)?;
        if escrow_balance < (self.payment + 2 * self.insurance) { // Double insurance for both seller and buyer.
            return Err("Insufficient funds in escrow.");
        }

        // Release the payment amount to the seller's account and update seller's balance.
        Escrow::release_funds(escrow_account, seller_account, escrow_authority_info, self.payment)?;
        seller.wallet.balance += self.payment;

        // Validate the seller's key.
        if entered_seller_key != self.seller_key {
            return Err("Invalid seller key provided.");
        }

        // Release the insurance amounts back to the seller and buyer, then update their balances.
        Escrow::release_funds(escrow_account, seller_account, escrow_authority_info, self.insurance)?;
        seller.wallet.balance += self.insurance;
        
        Escrow::release_funds(escrow_account, buyer_account, escrow_authority_info, self.insurance)?;
        buyer.wallet.balance += self.insurance;

        // Invalidate the keys.
        self.buyer_key.clear();
        self.seller_key.clear();

        // Update the status of the offer to 'Completed'.
        self.status = OfferStatus::Completed;

        // Mark the deal as successful for both the seller and buyer.
        seller.mark_deal(true);
        buyer.mark_deal(true);

        Ok(())
    }

    pub fn fail_offer(
        &mut self, 
        entered_seller_key: String,
        buyer: &mut User,
        escrow_account: &AccountInfo,
        penalty_account: &AccountInfo,
        escrow_authority_info: &AccountInfo,
    ) -> Result<(), &'static str> {
        // Ensure the offer is in the 'Accepted' state.
        if self.status != OfferStatus::Accepted {
            return Err("Offer is not in the 'Accepted' state.");
        }

        // Validate the seller's key.
        if entered_seller_key != self.seller_key {
            return Err("Invalid seller key provided.");
        }

        // Calculate the total amount to be transferred to the penalty account.
        let total_amount = self.payment + 2 * self.insurance; 

        // Transfer the total_amount from the escrow to the penalty account.
        Escrow::transfer_to_penalty(escrow_account, escrow_authority_info, total_amount)?;

        // Invalidate the keys.
        self.buyer_key.clear();
        self.seller_key.clear();

        // Update the status of the offer to 'Failed'.
        self.status = OfferStatus::Failed;

        // Mark the deal as failed for the buyer.
        buyer.mark_deal(false);

        Ok(())
    }

	pub fn expire_offer(
		&mut self,
		escrow_account: &AccountInfo,
		seller_account: &AccountInfo,
		buyer_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
	) -> Result<(), &'static str> {
		// Ensure the current date-time is past the meeting_datetime + 24 hours.
		let current_datetime = Utc::now();
		if current_datetime <= self.meeting_datetime + Duration::hours(24) {
			return Err("Offer hasn't expired yet.");
		}

		// Ensure the offer is still in the 'Accepted' state.
		if self.status != OfferStatus::Accepted {
			return Err("Offer is not in the 'Accepted' state.");
		}

		// Release the payment and buyer's insurance back to the buyer's account.
		let buyer_total = self.payment + self.insurance;
		Escrow::release_funds(escrow_account, buyer_account, escrow_authority_info, buyer_total)?;

		// Add the payment and insurance amounts back to the buyer's wallet.
		if let Some(buyer) = &mut self.buyer {
			buyer.wallet.balance += buyer_total;
		} else {
			return Err("Buyer not found in the offer.");
		}

		// Release the seller's insurance back to the seller's account.
		Escrow::release_funds(escrow_account, seller_account, escrow_authority_info, self.insurance)?;

		// Add the insurance amount back to the seller's wallet.
		self.seller.wallet.balance += self.insurance;

		// Update the status of the offer to 'Expired'.
		self.status = OfferStatus::Expired;

		Ok(())
	}
		
	pub fn cancel_offer(
		&mut self,
		seller_account: &AccountInfo,
		escrow_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
	) -> Result<(), &'static str> {
		// Ensure the offer is in the 'Listed' state.
		if self.status != OfferStatus::Listed {
			return Err("Offer is not in the 'Listed' state or has already been accepted.");
		}

		// Release the locked insurance back to the seller's account.
		// The locked amount in escrow is equal to the insurance amount, which is the same as the payment amount.
		Escrow::release_funds(escrow_account, seller_account, escrow_authority_info, self.insurance)?;

		// Invalidate the seller's key.
		self.seller_key.clear();

		// Update the status of the offer to 'Canceled'.
		self.status = OfferStatus::Canceled;

		Ok(())
	}

    /// Updates the status of the offer.
    pub fn update_status(&mut self, new_status: OfferStatus) {
        self.status = new_status;
    }
	
	/// Serializes the offer into a vector of bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        self.try_to_vec().map_err(|_| "Failed to serialize Offer")
    }

    /// Deserializes an offer from a slice of bytes.
    pub fn deserialize(input: &mut &[u8]) -> Result<Self, &'static str> {
        Self::try_from_slice(input).map_err(|_| "Failed to deserialize Offer")
    }
    
}
