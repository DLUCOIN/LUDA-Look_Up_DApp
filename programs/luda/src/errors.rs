use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DLUError {
    #[error("Invalid Instruction")]
    InvalidInstruction,

    #[error("Not Authorized")]
    NotAuthorized,

    #[error("Insufficient Funds")]
    InsufficientFunds,

    #[error("Offer Not Found")]
    OfferNotFound,

    #[error("Request Not Found")]
    RequestNotFound,

    #[error("Shipment Not Found")]
    ShipmentNotFound,

    #[error("Key Mismatch")]
    KeyMismatch,

    #[error("Operation Not Allowed")]
    OperationNotAllowed,

    #[error("Incorrect State")]
    IncorrectState,

    #[error("User Not Found")]
    UserNotFound,

    #[error("Invalid Operation")]
    InvalidOperation,

    #[error("Account Not Found")]
    AccountNotFound,

    #[error("Deserialization Failed")]
    DeserializationFailed,

    #[error("Serialization Failed")]
    SerializationFailed,

    #[error("Address Derivation Failed")]
    AddressDerivationFailed,

    #[error("Account Creation Failed")]
    AccountCreationFailed,

    #[error("Shipment Hasn't Expired Yet")]
    ShipmentNotExpired,

}

impl From<DLUError> for ProgramError {
    fn from(e: DLUError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
