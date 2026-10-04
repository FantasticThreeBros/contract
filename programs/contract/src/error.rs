use anchor_lang::prelude::*;

use anchor_lang::prelude::*;

#[error_code]
pub enum PassportError {
    #[msg("Signer is not the registry admin")]
    Unauthorized,

    #[msg("Registry is already initialized")]
    AlreadyInitialized,

    #[msg("Verifier is not active")]
    VerifierNotActive,

    #[msg("Identity is frozen")]
    IdentityFrozen,

    #[msg("Identity commitment already enrolled")]
    AlreadyEnrolled,

    #[msg("Nullifier already used for this application")]
    NullifierAlreadyUsed,

    #[msg("Invalid identity commitment")]
    InvalidCommitment,

    #[msg("Registry is globally frozen")]
    RegistryFrozen,
}

