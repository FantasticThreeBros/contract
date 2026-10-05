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

    #[msg("Credential account is not owned by the registered verifier program")]
    InvalidCredentialOwner,

    #[msg("Credential account data could not be parsed")]
    InvalidCredentialData,

    #[msg("Merkle proof has the wrong length for this tree depth")]
    InvalidProofLength,

    #[msg("Merkle proof does not match the current registry root")]
    InvalidMerkleProof,

    #[msg("The Merkle tree is full")]
    TreeFull,

    #[msg("Invalid tree depth")]
    InvalidTreeDepth,

    #[msg("Enrollment does not belong to the signing wallet")]
    WalletMismatch,

    #[msg("verify_personhood must be invoked via CPI from a consuming program")]
    NotCalledViaCpi,

    #[msg("consuming_program_id does not match the calling program")]
    ConsumerMismatch,

    #[msg("Credential was issued by a different network than the one this verifier trusts")]
    InvalidCredentialNetwork,

    #[msg("Credential is not active (frozen or revoked)")]
    CredentialNotActive,

    #[msg("Credential has expired")]
    CredentialExpired,
}
