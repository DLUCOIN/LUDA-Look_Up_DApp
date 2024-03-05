use crate::user::User;
use crate::onetimekeys::Onetimekeys;
use crate::dlu_wallet::Wallet;
use crate::escrow::Escrow;
use chrono::{DateTime, Utc};
use solana_program::borsh::{BorshSerialize, BorshDeserialize};


/// Represents an in-game location for shipment drop-offs and pickups.
pub struct Location {
    country: String,
    town: String,
    address: String,
}

/// Represents the current status of a shipment.
pub enum ShipmentStatus {
    Listed,
    Accepted,
    Completed,
    Failed,
    Expired,
    Canceled,
}

/// Represents a single shipment request posted by a sender.
pub struct Shipment {
    id: u64,
    status: ShipmentStatus,
    sender: User,
    carrier: Option<User>,
    recipient: User,
	pickup_point: Location,
    pickup_datetime: DateTime<Utc>,
    drop_off_point: Location,
    drop_off_datetime: DateTime<Utc>,
    payment: u64,
    insurance: u64,
    items_name: String,
    quantity: u32,
    sender_key: String,
    carrier_key: String,
    recipient_key: String,
    escrow_id: u64,
}

impl Shipment {
	/// List a new shipment request.
	pub fn list_shipment(
		id: u64,
		sender: &mut User,  // Mutable reference to sender for updating the wallet balance.
		recipient: User,    // Add recipient as an argument.
		items_name: String,
		quantity: u32,
		payment: u64,
		insurance: u64,     // Insurance set explicitly by sender.
		pickup_point: Location,           // New pickup location argument
		pickup_datetime: DateTime<Utc>,   // New pickup datetime argument
		drop_off_point: Location,
		drop_off_datetime: DateTime<Utc>,
	) -> Result<Self, &'static str> {

		// Check sender's balance for sufficient funds for payment.
		if sender.wallet.balance < payment {
			return Err("Insufficient funds for payment.");
		}

		// Deduct payment amount from sender's wallet.
		sender.wallet.balance -= payment;  // Assuming balance is mutable.

		// Lock payment amount in escrow.
		let escrow_id = Escrow::lock_funds(&sender.wallet, payment)?;

		Ok(Shipment {
			id,
			status: ShipmentStatus::Listed,
			sender: sender.clone(),
			carrier: None,
			recipient,  // Initialize recipient.
			pickup_point,           // Initialize pickup location
			pickup_datetime,        // Initialize pickup datetime
			drop_off_point,
			drop_off_datetime,
			payment,
			insurance,
			items_name,
			quantity,
			sender_key: String::new(),
			carrier_key: String::new(),
			recipient_key: String::new(),  // Initialize recipient's one-time key.
			escrow_id,
		})
	}

	pub fn accept_shipment(
		&mut self, 
		carrier: &mut User, // Mutable reference to the carrier.
		carrier_account: &AccountInfo, 
		escrow_account: &AccountInfo, 
		authority_info: &AccountInfo
	) -> Result<(), &'static str> {
		// Ensure the shipment is in the 'Listed' state.
		if self.status != ShipmentStatus::Listed {
			return Err("Shipment is not in the 'Listed' state.");
		}
		
		// Generate the one-time keys for sender, carrier, and recipient.
		self.sender_key = onetimekeys::generate_key(); 
		self.carrier_key = onetimekeys::generate_key();
		self.recipient_key = onetimekeys::generate_key();

		// Update the carrier field.
		self.carrier = Some(carrier.clone());

		// Check carrier's balance for insurance.
		let carrier_balance = DLUToken::get_balance(carrier_account)?;
		if carrier_balance < self.insurance {
			return Err("Insufficient funds in carrier's account for insurance.");
		}

		// Deduct the insurance amount from the carrier's wallet.
		carrier.wallet.balance -= self.insurance; // Assuming balance is mutable.

		// Lock the insurance amount in escrow.
		Escrow::lock_funds(carrier_account, escrow_account, authority_info, self.insurance)?;

		// Update the status of the shipment to 'Accepted'.
		self.status = ShipmentStatus::Accepted;

		Ok(())
	}

	pub fn complete_shipment(
		&mut self, 
		entered_carrier_key: String, 
		entered_recipient_key: String,
		sender_account: &AccountInfo,
		carrier_account: &AccountInfo,
		escrow_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
		sender: &mut User,  // Mutable reference to sender User
		carrier: &mut User, // Mutable reference to carrier User
	) -> Result<(), &'static str> {
		// Ensure the shipment is in the 'Accepted' state.
		if self.status != ShipmentStatus::Accepted {
			return Err("Shipment is not in the 'Accepted' state.");
		}

		// Validate the carrier's key.
		if entered_carrier_key != self.carrier_key {
			return Err("Invalid carrier key provided.");
		}

		// Check escrow balance.
		let escrow_balance = DLUToken::get_balance(escrow_account)?;
		if escrow_balance < (self.payment + self.insurance) {
			return Err("Insufficient funds in escrow.");
		}

		// Validate the recipient's key.
		if entered_recipient_key != self.recipient_key {
			return Err("Invalid recipient key provided.");
		}

		// Release the payment and insurance amounts to the carrier's account and update carrier's balance.
		let total_release = self.payment + self.insurance;
		Escrow::release_funds(escrow_account, carrier_account, escrow_authority_info, total_release)?;
		carrier.wallet.balance += total_release;

		// Invalidate the keys.
		self.sender_key.clear();
		self.carrier_key.clear();
		self.recipient_key.clear();

		// Update the status of the shipment to 'Completed'.
		self.status = ShipmentStatus::Completed;

		// Mark the shipment as successful for both the sender and carrier.
		sender.mark_deal(true);
		carrier.mark_deal(true);

		Ok(())
	}

	pub fn fail_shipment(
		&mut self, 
		entered_sender_key: String,
		carrier: &mut User,
		escrow_account: &AccountInfo,
		penalty_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
	) -> Result<(), &'static str> {
		// Ensure the shipment is in the 'Accepted' state.
		if self.status != ShipmentStatus::Accepted {
			return Err("Shipment is not in the 'Accepted' state.");
		}

		// Ensure that the carrier's key has been entered (i.e., the carrier has picked up the goods).
		if self.carrier_key.is_empty() {
			return Err("Carrier key has not been entered. Shipment has not been picked up.");
		}

		// Validate the sender's key.
		if entered_sender_key != self.sender_key {
			return Err("Invalid sender key provided.");
		}

		// Calculate the total amount to be transferred to the penalty account.
		let total_amount = self.payment + self.insurance; 

		// Transfer the total_amount from the escrow to the penalty account.
		Escrow::transfer_to_penalty(escrow_account, penalty_account, escrow_authority_info, total_amount)?;

		// Invalidate the keys.
		self.sender_key.clear();
		self.carrier_key.clear();
		self.recipient_key.clear();

		// Update the status of the shipment to 'Failed'.
		self.status = ShipmentStatus::Failed;

		// Mark the shipment as failed for the carrier.
		carrier.mark_deal(false);

		Ok(())
	}

	pub fn expire_shipment(
		&mut self,
		escrow_account: &AccountInfo,
		sender_account: &AccountInfo,
		carrier_account: &AccountInfo,
		escrow_authority_info: &AccountInfo,
	) -> Result<(), &'static str> {
		// Ensure the current date-time is past the drop_off_datetime + 24 hours.
		let current_datetime = Utc::now();
		if current_datetime <= self.drop_off_datetime + Duration::hours(24) {
			return Err("Shipment hasn't expired yet.");
		}

		// Ensure the shipment is still in the 'Accepted' state.
		if self.status != ShipmentStatus::Accepted {
			return Err("Shipment is not in the 'Accepted' state.");
		}

		// Release the payment back to the sender's account.
		Escrow::release_funds(escrow_account, sender_account, escrow_authority_info, self.payment)?;

		// Add the payment amount back to the sender's wallet.
		self.sender.wallet.balance += self.payment; // Assuming balance is mutable.

		// Release the carrier's insurance back to the carrier's account.
		Escrow::release_funds(escrow_account, carrier_account, escrow_authority_info, self.insurance)?;

		// Assuming the carrier is an Option<User>, and there is a possibility of it being None.
		if let Some(carrier) = &mut self.carrier {
			// Add the insurance amount back to the carrier's wallet.
			carrier.wallet.balance += self.insurance; // Assuming balance is mutable.
		} else {
			return Err("Carrier not found in the shipment.");
		}

		// Update the status of the shipment to 'Expired'.
		self.status = ShipmentStatus::Expired;

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
		// The locked amount in escrow is the payment amount.
		Escrow::release_funds(escrow_account, sender_account, escrow_authority_info, self.payment)?;

		// Invalidate the sender's key.
		self.sender_key.clear();

		// Update the status of the shipment to 'Canceled'.
		self.status = ShipmentStatus::Canceled;

		Ok(())
	}
	
	/// Updates the status of the shipment.
    pub fn update_status(&mut self, new_status: ShipmentStatus) {
        self.status = new_status;
    }
	
	/// Serializes the shipment into a vector of bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        self.try_to_vec().map_err(|_| "Failed to serialize Shipment")
    }

    /// Deserializes a shipment from a slice of bytes.
    pub fn deserialize(input: &mut &[u8]) -> Result<Self, &'static str> {
        Self::try_from_slice(input).map_err(|_| "Failed to deserialize Shipment")
    }
}
