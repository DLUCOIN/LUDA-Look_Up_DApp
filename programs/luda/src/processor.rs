use solana_program::{
    account_info::AccountInfo, 
    entrypoint::ProgramResult, 
    pubkey::Pubkey,
    program_error::ProgramError,
    program_pack::Pack,
    msg,
};
use crate::{
    user,
    offer,
    request,
    shipment,
    dlu_token,
    dlu_wallet,
    escrow,
    onetimekeys,
	addressing,
    error::DLUError,
};

pub struct Processor {}

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = DLUInstruction::unpack(input)?;

		match instruction {
			DLUInstruction::CreateUser { username } => {
				msg!("Instruction: CreateUser");

				// Derive a unique address for the user based on the username.
				let user_address = derive_user_address(program_id, &username)
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find or create the user account using the derived address.
				let user_account_info = match accounts.iter().find(|account| account.key == &user_address) {
					Some(account) => account,
					None => {
						create_account(&user_address).map_err(|_| DLUError::AccountCreationFailed)?
					}
				};

				// Create a new Wallet for the user. 
				let new_wallet = Wallet::new();

				// Create a new User instance using the derived public key from the user account.
				let new_user = User::new(username, *user_account_info.key, new_wallet);

				// Serialize the User.
				let serialized_user = new_user.serialize()?;

				// Save the serialized User to the Solana account.
				let mut user_data = &mut user_account_info.data.borrow_mut();
				user_data.copy_from_slice(&serialized_user);

				Ok(())
			},
	
			DLUInstruction::ListOffer { 
				id, 
				seller_account_key,
				goodsorservice_name, 
				goodsorservice_description, 
				payment, 
				meeting_point, 
				meeting_datetime 
			} => {
				msg!("Instruction: ListOffer");

				// Find the seller's account using the provided key
				let seller_account_info = accounts.iter().find(|account| account.key == seller_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the seller
				let mut seller_data = &mut seller_account_info.data.borrow_mut();
				let mut seller: User = User::deserialize(&mut seller_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Verify that the seller has enough funds for insurance
				if seller.wallet.balance < payment {
					return Err(DLUError::InsufficientFundsForInsurance);
				}

				// List a new Offer
				let new_offer = Offer::new(
					id,
					&mut seller,
					goodsorservice_name.clone(),
					goodsorservice_description.clone(),
					payment,
					meeting_point.clone(),
					meeting_datetime,
				).map_err(|_| DLUError::FailedToListOffer)?;

				// Serialize the Offer
				let serialized_offer = new_offer.serialize().map_err(|_| DLUError::SerializationFailed)?;

				// Derive the address of the offer using the id and ENTITY_OFFER
				let offer_address = derive_address(program_id, ENTITY_OFFER, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find or create the offer account using the derived address
				let offer_account_info = accounts.iter().find(|account| account.key == &offer_address)
					.ok_or(DLUError::OfferAccountNotFound)?;

				// Save the serialized Offer to the Solana account
				let mut offer_data = &mut offer_account_info.data.borrow_mut();
				offer_data.copy_from_slice(&serialized_offer);

				// Serialize and save the updated seller data
				let serialized_seller = seller.serialize().map_err(|_| DLUError::SerializationFailed)?;
				seller_data.copy_from_slice(&serialized_seller);

				Ok(())
			},
				
			DLUInstruction::AcceptOffer {
				id,
				buyer_account_key,
				escrow_account_key,
				authority_key
			} => {
				msg!("Instruction: AcceptOffer");

				let buyer_account_info = accounts.iter().find(|account| account.key == buyer_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;
				
				let authority_info = accounts.iter().find(|account| account.key == authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the buyer
				let mut buyer_data = &buyer_account_info.data.borrow_mut();
				let mut buyer: User = User::deserialize(&mut buyer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Derive the address of the offer using the id and ENTITY_OFFER
				let offer_address = derive_address(program_id, ENTITY_OFFER, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the offer account using the derived address
				let offer_account_info = accounts.iter().find(|account| account.key == &offer_address)
					.ok_or(DLUError::OfferNotFound)?;

				let mut offer_data = &mut offer_account_info.data.borrow_mut();
				let mut offer: Offer = Offer::deserialize(&mut offer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				offer.accept_offer(&mut buyer, buyer_account_info, escrow_account, authority_info)?;

				// Serialize the updated Offer and store it back into the Solana account
				let serialized_offer = offer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				offer_data.copy_from_slice(&serialized_offer);

				Ok(())
			},

			DLUInstruction::CompleteOffer {
				id,
				entered_buyer_key,
				entered_seller_key,
				seller_account_key,
				buyer_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: CompleteOffer");

				// Derive the address of the offer using the id and ENTITY_OFFER
				let offer_address = derive_address(program_id, ENTITY_OFFER, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the offer account using the derived address
				let offer_account_info = accounts.iter().find(|account| account.key == &offer_address)
					.ok_or(DLUError::OfferNotFound)?;

				let seller_account_info = accounts.iter().find(|account| account.key == seller_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let buyer_account_info = accounts.iter().find(|account| account.key == buyer_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the offer and users
				let mut offer_data = &offer_account_info.data.borrow_mut();
				let mut offer: Offer = Offer::deserialize(&mut offer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut seller_data = &seller_account_info.data.borrow_mut();
				let mut seller: User = User::deserialize(&mut seller_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut buyer_data = &buyer_account_info.data.borrow_mut();
				let mut buyer: User = User::deserialize(&mut buyer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the complete_offer method
				offer.complete_offer(
					entered_buyer_key, 
					entered_seller_key,
					seller_account_info,
					buyer_account_info,
					escrow_account_info,
					escrow_authority_info,
					&mut seller,
					&mut buyer,
				)?;

				// Serialize the updated offer and users back into their accounts
				let serialized_offer = offer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				offer_data[..serialized_offer.len()].copy_from_slice(&serialized_offer);

				let serialized_seller = seller.serialize().map_err(|_| DLUError::SerializationFailed)?;
				seller_data[..serialized_seller.len()].copy_from_slice(&serialized_seller);

				let serialized_buyer = buyer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				buyer_data[..serialized_buyer.len()].copy_from_slice(&serialized_buyer);

				Ok(())
			},

			DLUInstruction::FailOffer {
				id,
				entered_seller_key,
				buyer_account_key,
				escrow_account_key,
				penalty_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: FailOffer");

				// Derive the address of the offer using the id and ENTITY_OFFER
				let offer_address = derive_address(program_id, ENTITY_OFFER, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the offer account using the derived address
				let offer_account_info = accounts.iter().find(|account| account.key == &offer_address)
					.ok_or(DLUError::OfferNotFound)?;

				let buyer_account_info = accounts.iter().find(|account| account.key == buyer_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let penalty_account_info = accounts.iter().find(|account| account.key == penalty_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the offer and buyer
				let mut offer_data = &offer_account_info.data.borrow_mut();
				let mut offer: Offer = Offer::deserialize(&mut offer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut buyer_data = &buyer_account_info.data.borrow_mut();
				let mut buyer: User = User::deserialize(&mut buyer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Directly access the seller from the offer
				let mut seller = &mut offer.seller;

				// Call the fail_offer method
				offer.fail_offer(
					entered_seller_key, 
					&mut buyer,
					escrow_account_info,
					penalty_account_info,
					escrow_authority_info,
				)?;

				// Serialize the updated offer and buyer back into their accounts
				let serialized_offer = offer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				offer_data.copy_from_slice(&serialized_offer);

				let serialized_buyer = buyer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				buyer_data[..serialized_buyer.len()].copy_from_slice(&serialized_buyer);

				Ok(())

			},

			DLUInstruction::ExpireOffer {
				id,
				seller_account_key,
				buyer_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: ExpireOffer");

				// Derive the address of the offer using the id and ENTITY_OFFER
				let offer_address = derive_address(program_id, ENTITY_OFFER, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the offer account using the derived address
				let offer_account_info = accounts.iter().find(|account| account.key == &offer_address)
					.ok_or(DLUError::OfferNotFound)?;

				let seller_account_info = accounts.iter().find(|account| account.key == seller_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let buyer_account_info = accounts.iter().find(|account| account.key == buyer_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the offer
				let mut offer_data = &offer_account_info.data.borrow_mut();
				let mut offer: Offer = Offer::deserialize(&mut offer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the expire_offer method
				offer.expire_offer(
					escrow_account_info,
					seller_account_info,
					buyer_account_info,
					escrow_authority_info,
				)?;

				// Serialize the updated offer back into its account
				let serialized_offer = offer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				offer_data.copy_from_slice(&serialized_offer);

				Ok(())
			},

			DLUInstruction::CancelOffer {
				id,
				seller_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: CancelOffer");

				// Derive the address of the offer
				let offer_address = derive_address(program_id, ENTITY_OFFER, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the offer account using the derived address
				let offer_account_info = accounts.iter().find(|account| account.key == &offer_address)
					.ok_or(DLUError::OfferNotFound)?;

				let seller_account_info = accounts.iter().find(|account| account.key == &seller_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == &escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == &escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the offer
				let mut offer_data = &offer_account_info.data.borrow_mut();
				let mut offer: Offer = Offer::deserialize(&mut offer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the cancel_offer method
				offer.cancel_offer(
					seller_account_info,
					escrow_account_info,
					escrow_authority_info
				)?;

				// Serialize the updated offer and store it back into the Solana account
				let serialized_offer = offer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				offer_data.copy_from_slice(&serialized_offer);

				Ok(())
			}

			DLUInstruction::ListRequest {
				id,
				buyer_account_key,
				goodsorservice_name,
				goodsorservice_description,
				payment,
				meeting_point,
				meeting_datetime
			} => {
				msg!("Instruction: ListRequest");

				// Find the buyer's account using the provided key
				let buyer_account_info = accounts.iter().find(|account| account.key == buyer_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the buyer
				let mut buyer_data = &mut buyer_account_info.data.borrow_mut();
				let mut buyer: User = User::deserialize(&mut buyer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Verify that the buyer has enough funds for both the payment and insurance
				if buyer.wallet.balance < 2 * payment {
					return Err(DLUError::InsufficientFundsForPaymentAndInsurance);
				}

				// List a new Request
				let new_request = Request::list_request(
					id,
					&mut buyer,
					goodsorservice_name.clone(),
					goodsorservice_description.clone(),
					payment,
					meeting_point.clone(),
					meeting_datetime,
				).map_err(|_| DLUError::FailedToListRequest)?;

				// Serialize the Request
				let serialized_request = new_request.serialize().map_err(|_| DLUError::SerializationFailed)?;

				// Derive the address of the request using the id and ENTITY_REQUEST
				let request_address = derive_address(program_id, ENTITY_REQUEST, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find or create the request account using the derived address
				let request_account_info = accounts.iter().find(|account| account.key == &request_address)
					.ok_or(DLUError::RequestAccountNotFound)?;

				// Save the serialized Request to the Solana account
				let mut request_data = &mut request_account_info.data.borrow_mut();
				request_data.copy_from_slice(&serialized_request);

				// Serialize and save the updated buyer data
				let serialized_buyer = buyer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				buyer_data.copy_from_slice(&serialized_buyer);

				Ok(())
			},

			DLUInstruction::AcceptRequest { 
				id, 
				seller_account_key, 
				escrow_account_key, 
				authority_key 
			} => {
				msg!("Instruction: AcceptRequest");

				// Find the seller's account using the provided key
				let seller_account_info = accounts.iter().find(|account| account.key == seller_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Find the escrow account using the provided key
				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Find the authority account using the provided key
				let authority_info = accounts.iter().find(|account| account.key == authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the seller
				let mut seller_data = &mut seller_account_info.data.borrow_mut();
				let mut seller: User = User::deserialize(&mut seller_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Derive the address of the request using the id and ENTITY_REQUEST
				let request_address = derive_address(program_id, ENTITY_REQUEST, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the request account using the derived address
				let request_account_info = accounts.iter().find(|account| account.key == &request_address)
					.ok_or(DLUError::RequestNotFound)?;

				let mut request_data = &mut request_account_info.data.borrow_mut();
				let mut request: Request = Request::deserialize(&mut request_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				request.accept_request(&mut seller, seller_account_info, escrow_account_info, authority_info)?;

				// Serialize the updated Request and store it back into the Solana account
				let serialized_request = request.serialize().map_err(|_| DLUError::SerializationFailed)?;
				request_data.copy_from_slice(&serialized_request);

				Ok(())
			},

			DLUInstruction::CompleteRequest {
				id,
				entered_buyer_key,
				entered_seller_key,
				seller_account_key,
				buyer_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: CompleteRequest");

				// Derive the address of the request using the id and ENTITY_REQUEST
				let request_address = derive_address(program_id, ENTITY_REQUEST, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the request account using the derived address
				let request_account_info = accounts.iter().find(|account| account.key == &request_address)
					.ok_or(DLUError::RequestNotFound)?;

				let seller_account_info = accounts.iter().find(|account| account.key == seller_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let buyer_account_info = accounts.iter().find(|account| account.key == buyer_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the request and users
				let mut request_data = &request_account_info.data.borrow_mut();
				let mut request: Request = Request::deserialize(&mut request_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut seller_data = &seller_account_info.data.borrow_mut();
				let mut seller: User = User::deserialize(&mut seller_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut buyer_data = &buyer_account_info.data.borrow_mut();
				let mut buyer: User = User::deserialize(&mut buyer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the complete_request method
				request.complete_request(
					entered_buyer_key, 
					entered_seller_key,
					seller_account_info,
					buyer_account_info,
					escrow_account_info,
					escrow_authority_info,
					&mut seller,
					&mut buyer,
				)?;

				// Serialize the updated request and users back into their accounts
				let serialized_request = request.serialize().map_err(|_| DLUError::SerializationFailed)?;
				request_data[..serialized_request.len()].copy_from_slice(&serialized_request);

				let serialized_seller = seller.serialize().map_err(|_| DLUError::SerializationFailed)?;
				seller_data[..serialized_seller.len()].copy_from_slice(&serialized_seller);

				let serialized_buyer = buyer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				buyer_data[..serialized_buyer.len()].copy_from_slice(&serialized_buyer);

				Ok(())
			},

			DLUInstruction::FailRequest {
				id,
				entered_seller_key,
				buyer_account_key,
				escrow_account_key,
				penalty_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: FailRequest");

				// Derive the address of the request using the id and ENTITY_REQUEST
				let request_address = derive_address(program_id, ENTITY_REQUEST, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the request account using the derived address
				let request_account_info = accounts.iter().find(|account| account.key == &request_address)
					.ok_or(DLUError::RequestNotFound)?;

				let buyer_account_info = accounts.iter().find(|account| account.key == buyer_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let penalty_account_info = accounts.iter().find(|account| account.key == penalty_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the request and buyer
				let mut request_data = &request_account_info.data.borrow_mut();
				let mut request: Request = Request::deserialize(&mut request_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut buyer_data = &buyer_account_info.data.borrow_mut();
				let mut buyer: User = User::deserialize(&mut buyer_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Directly access the seller from the request
				let mut seller = &mut request.seller;

				// Call the fail_request method
				request.fail_request(
					entered_seller_key, 
					&mut buyer,
					escrow_account_info,
					penalty_account_info,
					escrow_authority_info,
				)?;

				// Serialize the updated request and buyer back into their accounts
				let serialized_request = request.serialize().map_err(|_| DLUError::SerializationFailed)?;
				request_data[..serialized_request.len()].copy_from_slice(&serialized_request);

				let serialized_buyer = buyer.serialize().map_err(|_| DLUError::SerializationFailed)?;
				buyer_data[..serialized_buyer.len()].copy_from_slice(&serialized_buyer);

				Ok(())

			},

			DLUInstruction::ExpireRequest {
				id,
				seller_account_key,
				buyer_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: ExpireRequest");

				// Derive the address of the request using the id and ENTITY_REQUEST
				let request_address = derive_address(program_id, ENTITY_REQUEST, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the request account using the derived address
				let request_account_info = accounts.iter().find(|account| account.key == &request_address)
					.ok_or(DLUError::RequestNotFound)?;

				let seller_account_info = accounts.iter().find(|account| account.key == seller_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let buyer_account_info = accounts.iter().find(|account| account.key == buyer_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the request
				let mut request_data = &request_account_info.data.borrow_mut();
				let mut request: Request = Request::deserialize(&mut request_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the expire_request method
				request.expire_request(
					escrow_account_info,
					seller_account_info,
					buyer_account_info,
					escrow_authority_info,
				)?;

				// Serialize the updated request back into its account
				let serialized_request = request.serialize().map_err(|_| DLUError::SerializationFailed)?;
				request_data.copy_from_slice(&serialized_request);

				Ok(())
			},

			DLUInstruction::CancelRequest {
				id,
				seller_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: CancelRequest");

				// Derive the address of the request
				let request_address = derive_address(program_id, ENTITY_REQUEST, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the request account using the derived address
				let request_account_info = accounts.iter().find(|account| account.key == &request_address)
					.ok_or(DLUError::RequestNotFound)?;

				let seller_account_info = accounts.iter().find(|account| account.key == &seller_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == &escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == &escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the request
				let mut request_data = &request_account_info.data.borrow_mut();
				let mut request: Request = Request::deserialize(&mut request_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the cancel_request method
				request.cancel_request(
					seller_account_info,
					escrow_account_info,
					escrow_authority_info
				)?;

				// Serialize the updated request and store it back into the Solana account
				let serialized_request = request.serialize().map_err(|_| DLUError::SerializationFailed)?;
				request_data.copy_from_slice(&serialized_request);

				Ok(())
			},

			DLUInstruction::ListShipment { 
				id, 
				sender_account_key,  // Sender's account key
				recipient,           // Recipient user
				items_name, 
				quantity,
				payment, 
				insurance,           // Explicit insurance set by sender
				drop_off_point, 
				drop_off_datetime 
			} => {
				msg!("Instruction: ListShipment");

				// Find the sender's account using the provided key
				let sender_account_info = accounts.iter().find(|account| account.key == sender_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the sender
				let mut sender_data = &mut sender_account_info.data.borrow_mut();
				let mut sender: User = User::deserialize(&mut sender_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Verify that the sender has enough funds for payment
				if sender.wallet.balance < payment {
					return Err(DLUError::InsufficientFundsForPayment);
				}

				// List a new Shipment
				let new_shipment = Shipment::list_shipment(
					id,
					&mut sender,
					recipient.clone(),
					items_name.clone(),
					quantity,
					payment,
					insurance,
					drop_off_point.clone(),
					drop_off_datetime,
				).map_err(|_| DLUError::FailedToListShipment)?;

				// Serialize the Shipment
				let serialized_shipment = new_shipment.serialize().map_err(|_| DLUError::SerializationFailed)?;

				// Derive the address of the shipment using the id and ENTITY_SHIPMENT
				let shipment_address = derive_address(program_id, ENTITY_SHIPMENT, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find or create the shipment account using the derived address
				let shipment_account_info = accounts.iter().find(|account| account.key == &shipment_address)
					.ok_or(DLUError::ShipmentAccountNotFound)?;

				// Save the serialized Shipment to the Solana account
				let mut shipment_data = &mut shipment_account_info.data.borrow_mut();
				shipment_data.copy_from_slice(&serialized_shipment);

				// Serialize and save the updated sender data
				let serialized_sender = sender.serialize().map_err(|_| DLUError::SerializationFailed)?;
				sender_data.copy_from_slice(&serialized_sender);

				Ok(())
			},

			DLUInstruction::AcceptShipment { 
				id, 
				carrier_account_key, 
				escrow_account_key,
				authority_key 
			} => {
				msg!("Instruction: AcceptShipment");

				// Find the carrier's account using the provided key
				let carrier_account_info = accounts.iter().find(|account| account.key == carrier_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let authority_info = accounts.iter().find(|account| account.key == authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the carrier
				let mut carrier_data = &carrier_account_info.data.borrow_mut();
				let mut carrier: User = User::deserialize(&mut carrier_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Derive the address of the shipment using the id and ENTITY_SHIPMENT
				let shipment_address = derive_address(program_id, ENTITY_SHIPMENT, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the shipment account using the derived address
				let shipment_account_info = accounts.iter().find(|account| account.key == &shipment_address)
					.ok_or(DLUError::ShipmentNotFound)?;

				let mut shipment_data = &mut shipment_account_info.data.borrow_mut();
				let mut shipment: Shipment = Shipment::deserialize(&mut shipment_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				shipment.accept_shipment(&mut carrier, carrier_account_info, escrow_account, authority_info)?;

				// Serialize the updated Shipment and store it back into the Solana account
				let serialized_shipment = shipment.serialize().map_err(|_| DLUError::SerializationFailed)?;
				shipment_data.copy_from_slice(&serialized_shipment);

				// Serialize and save the updated carrier data
				let serialized_carrier = carrier.serialize().map_err(|_| DLUError::SerializationFailed)?;
				carrier_data.copy_from_slice(&serialized_carrier);

				Ok(())
			},

			DLUInstruction::CompleteShipment { 
				id,
				entered_carrier_key,
				entered_recipient_key,
				sender_account_key,
				carrier_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: CompleteShipment");

				// Derive the address of the shipment using the id and ENTITY_SHIPMENT
				let shipment_address = derive_address(program_id, ENTITY_SHIPMENT, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the shipment account using the derived address
				let shipment_account_info = accounts.iter().find(|account| account.key == &shipment_address)
					.ok_or(DLUError::ShipmentNotFound)?;

				let sender_account_info = accounts.iter().find(|account| account.key == sender_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let carrier_account_info = accounts.iter().find(|account| account.key == carrier_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the shipment and users
				let mut shipment_data = &shipment_account_info.data.borrow_mut();
				let mut shipment: Shipment = Shipment::deserialize(&mut shipment_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut sender_data = &sender_account_info.data.borrow_mut();
				let mut sender: User = User::deserialize(&mut sender_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut carrier_data = &carrier_account_info.data.borrow_mut();
				let mut carrier: User = User::deserialize(&mut carrier_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the complete_shipment method
				shipment.complete_shipment(
					entered_carrier_key, 
					entered_recipient_key,
					sender_account_info,
					carrier_account_info,
					escrow_account_info,
					escrow_authority_info,
					&mut sender,
					&mut carrier,
				)?;

				// Serialize the updated shipment and users back into their accounts
				let serialized_shipment = shipment.serialize().map_err(|_| DLUError::SerializationFailed)?;
				shipment_data[..serialized_shipment.len()].copy_from_slice(&serialized_shipment);

				let serialized_sender = sender.serialize().map_err(|_| DLUError::SerializationFailed)?;
				sender_data[..serialized_sender.len()].copy_from_slice(&serialized_sender);

				let serialized_carrier = carrier.serialize().map_err(|_| DLUError::SerializationFailed)?;
				carrier_data[..serialized_carrier.len()].copy_from_slice(&serialized_carrier);

				Ok(())
			},

			DLUInstruction::FailShipment {
				id,
				entered_sender_key,
				carrier_account_key,
				escrow_account_key,
				penalty_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: FailShipment");

				// Derive the address of the shipment using the id and ENTITY_SHIPMENT
				let shipment_address = derive_address(program_id, ENTITY_SHIPMENT, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the shipment account using the derived address
				let shipment_account_info = accounts.iter().find(|account| account.key == &shipment_address)
					.ok_or(DLUError::ShipmentNotFound)?;

				let carrier_account_info = accounts.iter().find(|account| account.key == carrier_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let penalty_account_info = accounts.iter().find(|account| account.key == penalty_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the shipment and carrier
				let mut shipment_data = &shipment_account_info.data.borrow_mut();
				let mut shipment: Shipment = Shipment::deserialize(&mut shipment_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				let mut carrier_data = &carrier_account_info.data.borrow_mut();
				let mut carrier: User = User::deserialize(&mut carrier_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the fail_shipment method
				shipment.fail_shipment(
					entered_sender_key, 
					&mut carrier,
					escrow_account_info,
					penalty_account_info,
					escrow_authority_info,
				)?;

				// Serialize the updated shipment and carrier back into their accounts
				let serialized_shipment = shipment.serialize().map_err(|_| DLUError::SerializationFailed)?;
				shipment_data[..serialized_shipment.len()].copy_from_slice(&serialized_shipment);

				let serialized_carrier = carrier.serialize().map_err(|_| DLUError::SerializationFailed)?;
				carrier_data[..serialized_carrier.len()].copy_from_slice(&serialized_carrier);

				Ok(())
			},

			DLUInstruction::ExpireShipment {
				id,
				sender_account_key,
				carrier_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: ExpireShipment");

				// Derive the address of the shipment using the id and ENTITY_SHIPMENT
				let shipment_address = derive_address(program_id, ENTITY_SHIPMENT, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the shipment account using the derived address
				let shipment_account_info = accounts.iter().find(|account| account.key == &shipment_address)
					.ok_or(DLUError::ShipmentNotFound)?;

				let sender_account_info = accounts.iter().find(|account| account.key == sender_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let carrier_account_info = accounts.iter().find(|account| account.key == carrier_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the shipment
				let mut shipment_data = &shipment_account_info.data.borrow_mut();
				let mut shipment: Shipment = Shipment::deserialize(&mut shipment_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the expire_shipment method
				shipment.expire_shipment(
					escrow_account_info,
					sender_account_info,
					carrier_account_info,
					escrow_authority_info,
				)?;

				// Serialize the updated shipment back into its account
				let serialized_shipment = shipment.serialize().map_err(|_| DLUError::SerializationFailed)?;
				shipment_data.copy_from_slice(&serialized_shipment);

				Ok(())
			},

			DLUInstruction::CancelShipment {
				id,
				sender_account_key,
				escrow_account_key,
				escrow_authority_key,
			} => {
				msg!("Instruction: CancelShipment");

				// Derive the address of the shipment
				let shipment_address = derive_address(program_id, ENTITY_SHIPMENT, &id.to_string())
					.map_err(|_| DLUError::AddressDerivationFailed)?;

				// Find the shipment account using the derived address
				let shipment_account_info = accounts.iter().find(|account| account.key == &shipment_address)
					.ok_or(DLUError::ShipmentNotFound)?;

				let sender_account_info = accounts.iter().find(|account| account.key == &sender_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_account_info = accounts.iter().find(|account| account.key == &escrow_account_key)
					.ok_or(DLUError::AccountNotFound)?;

				let escrow_authority_info = accounts.iter().find(|account| account.key == &escrow_authority_key)
					.ok_or(DLUError::AccountNotFound)?;

				// Deserialize the shipment
				let mut shipment_data = &shipment_account_info.data.borrow_mut();
				let mut shipment: Shipment = Shipment::deserialize(&mut shipment_data)
					.map_err(|_| DLUError::DeserializationFailed)?;

				// Call the cancel_shipment method
				shipment.cancel_shipment(
					sender_account_info,
					escrow_account_info,
					escrow_authority_info
				)?;

				// Serialize the updated shipment and store it back into the Solana account
				let serialized_shipment = shipment.serialize().map_err(|_| DLUError::SerializationFailed)?;
				shipment_data.copy_from_slice(&serialized_shipment);

				Ok(())
			},

			_ => {
				msg!("Error: Unhandled Instruction");
				return Err(DLUError::UnhandledInstruction.into());
			}
		}

		Ok(())
	}
	
}
	
#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum DLUInstruction {

    CreateUser {
        username: String,
    },

    ListOffer {
        id: String,
        seller_account_key: Pubkey,
        goodsorservice_name: String,
        goodsorservice_description: String,
        payment: u64,
        meeting_point: String,
        meeting_datetime: i64, // Datetime is represented as a Unix timestamp
    },

    AcceptOffer {
        id: String,
        buyer_account_key: Pubkey,
        escrow_account_key: Pubkey,
        authority_key: Pubkey,
    },

    CompleteOffer {
        id: String,
        entered_buyer_key: String,
        entered_seller_key: String,
        seller_account_key: Pubkey,
        buyer_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    FailOffer {
        id: String,
        entered_seller_key: String,
        buyer_account_key: Pubkey,
        escrow_account_key: Pubkey,
        penalty_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    ExpireOffer {
        id: String,
        seller_account_key: Pubkey,
        buyer_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    CancelOffer {
        id: String,
        seller_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },
	
    ListRequest {
        id: String,
        buyer_account_key: Pubkey,
        goodsorservice_name: String,
        goodsorservice_description: String,
        payment: u64,
        meeting_point: String,
        meeting_datetime: String,
    },

    AcceptRequest {
        id: String,
        seller_account_key: Pubkey,
        escrow_account_key: Pubkey,
        authority_key: Pubkey,
    },

    CompleteRequest {
        id: String,
        entered_buyer_key: String,
        entered_seller_key: String,
        seller_account_key: Pubkey,
        buyer_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    FailRequest {
        id: String,
        entered_seller_key: String,
        buyer_account_key: Pubkey,
        escrow_account_key: Pubkey,
        penalty_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    ExpireRequest {
        id: String,
        seller_account_key: Pubkey,
        buyer_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    CancelRequest {
        id: String,
        seller_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },
	
	ListShipment {
        id: String,
        sender_account_key: Pubkey,
        recipient: User,
        items_name: String,
        quantity: u64,
        payment: u64,
        insurance: u64,
        drop_off_point: String,
        drop_off_datetime: String,
    },

    AcceptShipment {
        id: String,
        carrier_account_key: Pubkey,
        escrow_account_key: Pubkey,
        authority_key: Pubkey,
    },

    CompleteShipment {
        id: String,
        entered_carrier_key: String,
        entered_recipient_key: String,
        sender_account_key: Pubkey,
        carrier_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    FailShipment {
        id: String,
        entered_sender_key: String,
        carrier_account_key: Pubkey,
        escrow_account_key: Pubkey,
        penalty_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    ExpireShipment {
        id: String,
        sender_account_key: Pubkey,
        carrier_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },

    CancelShipment {
        id: String,
        sender_account_key: Pubkey,
        escrow_account_key: Pubkey,
        escrow_authority_key: Pubkey,
    },
}

impl DLUInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(input)
            .map_err(|_| ProgramError::InvalidInstructionData)
    }
}

