use solana_program::pubkey::Pubkey;

// Constants representing different entity types in the system.
pub const ENTITY_OFFER: &str = "offer";
pub const ENTITY_REQUEST: &str = "request";
pub const ENTITY_SHIPMENT: &str = "shipment";

/// Derives an address based on the provided program_id, entity type, and entity ID.
pub fn derive_address(program_id: &Pubkey, entity_type: &str, entity_id: &str) -> Result<Pubkey, &'static str> {
    // Validate entity type
    match entity_type {
        ENTITY_OFFER | ENTITY_REQUEST | ENTITY_SHIPMENT => {},
        _ => return Err("Invalid entity type"),
    }

    // Validate entity ID length to avoid too long seeds
    if entity_id.len() > 32 {
        return Err("Entity ID too long");
    }

    let seed = format!("{}{}", entity_type, entity_id);
    Pubkey::create_with_seed(program_id, &seed, &program_id).map_err(|_| "Failed to derive address")
}

pub enum EntityType {
    Offer,
    Request,
    Shipment,
}

pub struct AcceptedEntity {
    entity_address: Pubkey,
    entity_type: EntityType,
    seller_or_sender: Pubkey,
    buyer_or_carrier: Pubkey,
    recipient: Option<Pubkey>, // This field will be Some(Pubkey) for shipments and None for offers/requests.
}

pub struct IndexAccount {
    pub active_offers: Vec<Pubkey>,
    pub accepted_offers: Vec<AcceptedEntity>,
    pub history_offers: Vec<Pubkey>,
    
    pub active_requests: Vec<Pubkey>,
    pub accepted_requests: Vec<AcceptedEntity>,
    pub history_requests: Vec<Pubkey>,
    
    pub active_shipments: Vec<Pubkey>,
    pub accepted_shipments: Vec<AcceptedEntity>,
    pub history_shipments: Vec<Pubkey>,
}

impl IndexAccount {
    /// Creates a new index account with empty lists.
    pub fn new() -> Self {
        IndexAccount {
            active_offers: Vec::new(),
            accepted_offers: Vec::new(),
            history_offers: Vec::new(),
            
            active_requests: Vec::new(),
            accepted_requests: Vec::new(),
            history_requests: Vec::new(),
            
            active_shipments: Vec::new(),
            accepted_shipments: Vec::new(),
            history_shipments: Vec::new(),
        }
    }

     // OFFERS
    pub fn add_offer(&mut self, offer_address: Pubkey) {
        self.active_offers.push(offer_address);
    }

    pub fn remove_offer(&mut self, offer_address: &Pubkey) {
        self.active_offers.retain(|&x| x != *offer_address);
    }

    pub fn accept_offer(&mut self, offer_address: &Pubkey, seller: &Pubkey, buyer: &Pubkey) {
        self.remove_offer(offer_address);
        let accepted_offer = AcceptedEntity {
            entity_address: *offer_address,
            seller: *seller,
            buyer_or_carrier: *buyer,
        };
        self.accepted_offers.push(accepted_offer);
    }

    pub fn move_offer_to_history(&mut self, offer_address: &Pubkey) {
        self.history_offers.push(*offer_address);
    }
	
	pub fn cancel_offer(&mut self, offer_address: &Pubkey) {
		self.remove_offer(offer_address);
	}

	// REQUESTS
	pub fn add_request(&mut self, request_address: Pubkey) {
		self.active_requests.push(request_address);
	}

	pub fn remove_request(&mut self, request_address: &Pubkey) {
		self.active_requests.retain(|&x| x != *request_address);
	}

	pub fn accept_request(&mut self, request_address: &Pubkey, buyer: &Pubkey, seller: &Pubkey) {
		self.remove_request(request_address);
		let accepted_request = AcceptedEntity {
			entity_address: *request_address,
			seller: *seller,
			buyer_or_carrier: *buyer,
		};
		self.accepted_requests.push(accepted_request);
	}

	pub fn move_request_to_history(&mut self, request_address: &Pubkey) {
		self.history_requests.push(*request_address);
	}

	pub fn cancel_request(&mut self, request_address: &Pubkey) {
		self.remove_request(request_address);
	}


    /// Adds a request's address to the active requests list.
    pub fn add_request(&mut self, request_address: Pubkey) -> Result<(), &'static str> {
        if self.active_requests.contains(&request_address) {
            return Err("Request already exists in the list");
        }
        self.active_requests.push(request_address);
        Ok(())
    }

	// SHIPMENTS
	pub fn add_shipment(&mut self, shipment_address: Pubkey) {
		self.active_shipments.push(shipment_address);
	}

	pub fn remove_shipment(&mut self, shipment_address: &Pubkey) {
		self.active_shipments.retain(|&x| x != *shipment_address);
	}

	pub fn accept_shipment(&mut self, shipment_address: &Pubkey, sender: &Pubkey, carrier: &Pubkey, recipient: &Pubkey) {
		self.remove_shipment(shipment_address);
		let accepted_shipment = AcceptedShipment {
			shipment_address: *shipment_address,
			sender: *sender,
			carrier: *carrier,
			recipient: *recipient,
		};
		self.accepted_shipments.push(accepted_shipment);
	}

	pub fn move_shipment_to_history(&mut self, shipment_address: &Pubkey) {
		self.history_shipments.push(*shipment_address);
	}

	pub fn cancel_shipment(&mut self, shipment_address: &Pubkey) {
		self.remove_shipment(shipment_address);
	}

}
