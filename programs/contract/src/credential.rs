//! Credential account deserialization, isolated so it is a one-file swap.
//!
//! TODO: the on-chain account layout of the Reclaim Protocol verifier is not
//! confirmed yet. Replace `ReclaimCredential::parse` once it is, see
//! https://docs.reclaimprotocol.org/solana/quickstart
//!
//! Until then the stub accepts any data. That is safe ONLY because the
//! caller has already checked `credential_account.owner ==
//! verifier_config.verifier_program`; the layout parse is for extra checks
//! (valid/unexpired proof, and binding the credential to the enrolling wallet
//! so nobody can enroll with another person's credential account).

use anchor_lang::prelude::*;

use crate::error::PassportError;

pub trait CredentialReader: Sized {
    fn parse(data: &[u8]) -> Result<Self>;
    /// Wallet the credential was issued to, if the layout exposes it.
    fn subject(&self) -> Option<Pubkey>;
}

/// Stub for the Reclaim credential layout (see module docs).
pub struct ReclaimCredential;

impl CredentialReader for ReclaimCredential {
    fn parse(_data: &[u8]) -> Result<Self> {
        // TODO(reclaim): deserialize the real layout; return
        // PassportError::InvalidCredentialData on mismatch.
        let _ = PassportError::InvalidCredentialData;
        Ok(ReclaimCredential)
    }

    fn subject(&self) -> Option<Pubkey> {
        None
    }
}
