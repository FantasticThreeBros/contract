//! Credential account deserialization, isolated so a different verifier is a one-file swap.
//!
//! Implemented for Civic Pass, whose "gateway token" is an on-chain account owned by the
//! Civic gateway program (`gatem74V238djXdzWnJf94Wo1DcnuGkfijbf3AuBhfs`, same id on devnet
//! and mainnet). Layout source: github.com/civicteam/on-chain-identity-gateway,
//! solana/program/src/state.rs (borsh; Option fields are variable length, and the account
//! is zero-padded to 109 bytes, so we parse incrementally and ignore trailing bytes).
//!
//! (Reclaim Protocol was evaluated first, but its Solana program is stateless: it verifies a
//! proof inside the transaction and leaves no credential account to read.)
//!
//! This module only parses. Which program owns the account and which gatekeeper network
//! is trusted are checked by the caller against `VerifierConfig`: owner-checking alone is
//! NOT enough, since anyone can create their own gatekeeper network and issue passes owned
//! by the same gateway program.

use anchor_lang::prelude::*;

use crate::error::PassportError;

/// What enroll_identity needs to know about a credential, verifier-agnostic.
pub struct CredentialInfo {
    /// Wallet the credential was issued to.
    pub subject: Pubkey,
    /// Issuing network / pass type (pinned in `VerifierConfig`).
    pub network: Pubkey,
    pub is_active: bool,
    /// Unix timestamp, if the credential expires.
    pub expire_time: Option<i64>,
}

pub trait CredentialReader {
    fn parse(data: &[u8]) -> Result<CredentialInfo>;
}

#[derive(AnchorDeserialize)]
enum GatewayTokenState {
    Active,
    Frozen,
    Revoked,
}

/// Mirror of Civic's `GatewayToken` (field order matters, it is borsh).
#[derive(AnchorDeserialize)]
struct GatewayToken {
    _version: u8,
    _parent_gateway_token: Option<Pubkey>, // deprecated
    owner_wallet: Pubkey,
    _owner_identity: Option<Pubkey>, // deprecated
    gatekeeper_network: Pubkey,
    _issuing_gatekeeper: Pubkey,
    state: GatewayTokenState,
    expire_time: Option<i64>,
}

pub struct CivicGatewayToken;

impl CredentialReader for CivicGatewayToken {
    fn parse(data: &[u8]) -> Result<CredentialInfo> {
        let mut slice = data;
        let token = <GatewayToken as AnchorDeserialize>::deserialize(&mut slice)
            .map_err(|_| error!(PassportError::InvalidCredentialData))?;
        Ok(CredentialInfo {
            subject: token.owner_wallet,
            network: token.gatekeeper_network,
            is_active: matches!(token.state, GatewayTokenState::Active),
            expire_time: token.expire_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    // Real gateway token accounts read from devnet (program gatem74V...).
    // Active, expired in 2022, devnet CAPTCHA network, version byte = 2.
    const REAL_ACTIVE: &str = "02007d363742cf25d4cbf45c37bd05edc662904cc87e85b562fb0c1eacd3d7d2c7eb000d3d9d16c2282b3163c2dbbc7cff131ad6c2269926c41268dcd76929479c0642df1eaf6ffcf6cdc4de124d748e1e8cee11c46f215d00bd0afa78f8c138376333000101fd206200000000";
    // Revoked (state = 2), expires far in the future.
    const REAL_REVOKED: &str = "0000540c563db38804a6f652e5caa27162a6a23adfa0009784eacb9a7bf3abf694bb000d4c4d9715faa3d078c5b58dcf0752bebee422ee35342e7be124dd682e05d89d0d390358bcd4886b50e659313a0376d88f937923bac5a7079ed7dca756525284020183fe309800000000";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn parses_real_active_token() {
        let c = CivicGatewayToken::parse(&unhex(REAL_ACTIVE)).unwrap();
        assert_eq!(c.subject, Pubkey::from_str("9RmuqctYTbegNjXroB3FAsS1Q8ebV2wxLp286H4YuRhC").unwrap());
        assert_eq!(c.network, Pubkey::from_str("tgnuXXNMDLK8dy7Xm1TdeGyc95MDym4bvAQCwcW21Bf").unwrap());
        assert!(c.is_active);
        assert_eq!(c.expire_time, Some(1646329089));
    }

    #[test]
    fn parses_real_revoked_token() {
        let c = CivicGatewayToken::parse(&unhex(REAL_REVOKED)).unwrap();
        assert_eq!(c.subject, Pubkey::from_str("6f6A8XdbN5pLXASbkXvBUZfmgCABMvJ7wDQRH9tbEea6").unwrap());
        assert_eq!(c.network, Pubkey::from_str("tunQheuPpHhjjsbrUDp4rikqYez9UXv4SXLRHf9Kzsv").unwrap());
        assert!(!c.is_active);
    }

    #[test]
    fn tolerates_none_expiry_and_trailing_padding() {
        // version, parent None, owner, identity None, network, gatekeeper, Active, expire None, zero pad
        let mut d = vec![0u8, 0];
        d.extend([1u8; 32]);
        d.push(0);
        d.extend([2u8; 32]);
        d.extend([3u8; 32]);
        d.extend([0u8, 0u8]);
        d.resize(109, 0);
        let c = CivicGatewayToken::parse(&d).unwrap();
        assert_eq!(c.subject, Pubkey::new_from_array([1; 32]));
        assert_eq!(c.network, Pubkey::new_from_array([2; 32]));
        assert!(c.is_active && c.expire_time.is_none());
    }

    #[test]
    fn rejects_garbage() {
        assert!(CivicGatewayToken::parse(&[0u8; 5]).is_err());
        assert!(CivicGatewayToken::parse(&[]).is_err());
    }
}
