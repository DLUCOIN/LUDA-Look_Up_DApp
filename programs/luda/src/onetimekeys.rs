use uuid::Uuid;
use std::collections::HashMap;

// Represents keys for a deal.
pub struct DealKeys {
    seller_key: String,
    buyer_key: String,
}

// Represents keys for a shipment.
pub struct ShipmentKeys {
    sender_key: String,
    carrier_key: String,
    recipient_key: String,
}

pub struct KeyManager {
    deal_keys: HashMap<u64, DealKeys>,       // Maps deal ID to its keys.
    shipment_keys: HashMap<u64, ShipmentKeys>, // Maps shipment ID to its keys.
    used_keys: HashMap<String, bool>,        // Tracks if a key has been used.
}

impl KeyManager {
    pub fn new() -> Self {
        KeyManager {
            deal_keys: HashMap::new(),
            shipment_keys: HashMap::new(),
            used_keys: HashMap::new(),
        }
    }

    // Generates keys for a deal.
    pub fn generate_deal_keys(&mut self, deal_id: u64) -> &DealKeys {
        let keys = DealKeys {
            seller_key: Uuid::new_v4().to_string(),
            buyer_key: Uuid::new_v4().to_string(),
        };
        self.deal_keys.insert(deal_id, keys);
        self.deal_keys.get(&deal_id).unwrap()
    }

    // Generates keys for a shipment.
    pub fn generate_shipment_keys(&mut self, shipment_id: u64) -> &ShipmentKeys {
        let keys = ShipmentKeys {
            sender_key: Uuid::new_v4().to_string(),
            carrier_key: Uuid::new_v4().to_string(),
            recipient_key: Uuid::new_v4().to_string(),
        };
        self.shipment_keys.insert(shipment_id, keys);
        self.shipment_keys.get(&shipment_id).unwrap()
    }

    // Validates a key for a deal or shipment and marks it as used.
    pub fn validate_and_use_key(&mut self, key: &str) -> bool {
        if self.used_keys.contains_key(key) {
            return false; // Key was already used.
        }

        let is_valid = self.deal_keys.values().any(|deal_keys| deal_keys.seller_key == key || deal_keys.buyer_key == key) ||
                       self.shipment_keys.values().any(|shipment_keys| shipment_keys.sender_key == key || 
                                                       shipment_keys.carrier_key == key || 
                                                       shipment_keys.recipient_key == key);

        if is_valid {
            self.used_keys.insert(key.to_string(), true);
            // Invalidate the key in original maps
            for deal_keys in self.deal_keys.values_mut() {
                if deal_keys.seller_key == key {
                    deal_keys.seller_key.clear();
                }
                if deal_keys.buyer_key == key {
                    deal_keys.buyer_key.clear();
                }
            }
            for shipment_keys in self.shipment_keys.values_mut() {
                if shipment_keys.sender_key == key {
                    shipment_keys.sender_key.clear();
                }
                if shipment_keys.carrier_key == key {
                    shipment_keys.carrier_key.clear();
                }
                if shipment_keys.recipient_key == key {
                    shipment_keys.recipient_key.clear();
                }
            }
        }

        is_valid
    }
    
}
