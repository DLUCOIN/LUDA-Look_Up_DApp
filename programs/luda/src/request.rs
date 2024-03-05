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

/// Represents the current status of a request.
pub enum RequestStatus {
    Listed,
    Accepted,
    Completed,
    Failed,
    Expired,
    Canceled,
}

/// Represents a single request posted by a buyer.
pub struct Request {
    id: u64,
    status: RequestStatus,
    buyer: User,
    seller: Option<User>,
    meeting_point: Location,
    meeting_datetime: DateTime<Utc>,
    payment: u64,
    insurance: u64,
    goodsorservice_name: String,
    goodsorservice_description: String,
    buyer_key: String,
    seller_key: String,
    escrow_id: u64,
}

impl Request {
    /// List a new request.
    pub fn list_request(
        id: u64,
        buyer: &mut User,
        goodsorservice_name: String,
        goodsorservice_description: String,
        payment: u64,
        meeting_point: Location,
        meeting_datetime: DateTime<Utc>,
    ) -> Result<Self, &'static str> {
        // Insurance is always equal to payment.
        let insurance = payment;

        // Check buyer's balance for sufficient funds.
        if buyer.wallet.balance < (payment + insurance) {
            return Err("Insufficient funds in buyer's wallet.");
        }

        // Deduct payment and insurance amounts from buyer's wallet.
        buyer.wallet.balance -= (payment + insurance);

        // Lock payment and insurance amounts in escrow.
        let escrow_id = Escrow::lock_funds(&buyer.wallet, payment + insurance)?;

        Ok(Request {
            id,
            status: RequestStatus::Listed,
            buyer: buyer.clone(),
            seller: None,
            meeting_point,
            meeting_datetime,
            payment,
            insurance,
            goodsorservice_name,
            goodsorservice_description,
            buyer_key: String::new(),
            seller_key: String::new(),
            escrow_id,
        })
    }

    /// Accepts a request by a seller.
	pub fn accept_request(
		&mut self, 
		seller: &mut User,
		seller_account: &AccountInfo, 
		escrow_account: &AccountInfo, 
		authority_info: &AccountInfo
	) -> Result<(), &'static str> {
		// Ensure the request is in the 'Listed' state.
		if self.status != RequestStatus::Listed {
			return Err("Request is not in the 'Listed' state.");
		}
		
		// Generate the one-time keys for both buyer and seller.
		self.buyer_key = onetimekeys::generate_key(); 
		self.seller_key = onetimekeys::generate_key();

		// Update the seller field.
		self.seller = Some(seller.clone());

		// Check seller's balance for sufficient funds.
		if seller.wallet.balance < self.insurance { 
			return Err("Insufficient funds in seller's wallet for insurance.");
		}

		// Deduct insurance amount from seller's wallet.
		seller.wallet.balance -= self.insurance;

		// Lock the insurance amount in escrow.
		let _escrow_id = Escrow::lock_funds(&seller.wallet, self.insurance)?;

		// Update the status of the request to 'Accepted'.
		self.status = RequestStatus::Accepted;

		Ok(())
	}

	pub fn complete_request(
		&mut self, 
		entered_buyer_key: String, 
		entered_seller_key: String,
		seller_account: &AccountInfo,
		buyer_account: &AccountInfo,
		escrow_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
		seller: &mut User,
		buyer: &mut User 
	) -> Result<(), &'static str> {
		// Ensure the request is in the 'Accepted' state.
		if self.status != RequestStatus::Accepted {
			return Err("Request is not in the 'Accepted' state.");
		}

		// Validate the buyer's key.
		if entered_buyer_key != self.buyer_key {
			return Err("Invalid buyer key provided.");
		}

		// Check escrow balance.
		let escrow_balance = DLUToken::get_balance(escrow_account)?;
		if escrow_balance < (self.payment + 2 * self.insurance) { 
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

		// Update the status of the request to 'Completed'.
		self.status = RequestStatus::Completed;

		// Mark the deal as successful for both the seller and buyer.
		seller.mark_deal(true);
		buyer.mark_deal(true);

		Ok(())
	}

	pub fn fail_request(
		&mut self, 
		entered_seller_key: String,
		buyer: &mut User,
		escrow_account: &AccountInfo,
		penalty_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
	) -> Result<(), &'static str> {
		// Ensure the request is in the 'Accepted' state.
		if self.status != RequestStatus::Accepted {
			return Err("Request is not in the 'Accepted' state.");
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

		// Update the status of the request to 'Failed'.
		self.status = RequestStatus::Failed;

		// Mark the deal as failed for the buyer.
		buyer.mark_deal(false);

		Ok(())
	}

	pub fn expire_request(
		&mut self,
		escrow_account: &AccountInfo,
		seller_account: &AccountInfo,
		buyer_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
	) -> Result<(), &'static str> {
		// Ensure the current date-time is past the meeting_datetime + 24 hours.
		let current_datetime = Utc::now();
		if current_datetime <= self.meeting_datetime + Duration::hours(24) {
			return Err("Request hasn't expired yet.");
		}

		// Ensure the request is still in the 'Accepted' state.
		if self.status != RequestStatus::Accepted {
			return Err("Request is not in the 'Accepted' state.");
		}

		// Release the payment and buyer's insurance back to the buyer's account.
		let buyer_total = self.payment + self.insurance;
		Escrow::release_funds(escrow_account, buyer_account, escrow_authority_info, buyer_total)?;

		// Add the payment and insurance amounts back to the buyer's wallet.
		if let Some(buyer) = &mut self.buyer {
			buyer.wallet.balance += buyer_total;
		} else {
			return Err("Buyer not found in the request.");
		}

		// Release the seller's insurance back to the seller's account.
		Escrow::release_funds(escrow_account, seller_account, escrow_authority_info, self.insurance)?;

		// Add the insurance amount back to the seller's wallet.
		self.seller.wallet.balance += self.insurance;

		// Update the status of the request to 'Expired'.
		self.status = RequestStatus::Expired;

		Ok(())
	}

	pub fn cancel_request(
		&mut self,
		seller_account: &AccountInfo,
		escrow_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
	) -> Result<(), &'static str> {
		// Ensure the request is in the 'Listed' state.
		if self.status != RequestStatus::Listed {
			return Err("Request is not in the 'Listed' state or has already been accepted.");
		}

		// Release the locked insurance back to the seller's account.
		// The locked amount in escrow is equal to the insurance amount, which is the same as the payment amount.
		Escrow::release_funds(escrow_account, seller_account, escrow_authority_info, self.insurance)?;

		// Invalidate the seller's key.
		self.seller_key.clear();

		// Update the status of the request to 'Canceled'.
		self.status = RequestStatus::Canceled;

		Ok(())
	}

    /// Updates the status of the request.
    pub fn update_status(&mut self, new_status: RequestStatus) {
        self.status = new_status;
    }
	
	/// Serializes the request into a vector of bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        self.try_to_vec().map_err(|_| "Failed to serialize Request")
    }

    /// Deserializes a request from a slice of bytes.
    pub fn deserialize(input: &mut &[u8]) -> Result<Self, &'static str> {
        Self::try_from_slice(input).map_err(|_| "Failed to deserialize Request")
    }
    
}