use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct RegistryConfig {
    pub admin: Pubkey,
    pub is_frozen: bool,
    pub merkle_root: [u8; 32],
    pub tree_depth: u8,
    pub bump: u8,
    /// Index of the next free leaf. enroll_identity needs it to know which tree
    /// path the client-supplied proof must correspond to.
    pub next_leaf_index: u32,
}

#[account]
#[derive(InitSpace)]
pub struct VerifierConfig {
    pub verifier_key: Pubkey,
    pub is_active: bool,
    pub bump: u8,
}

/// One per unique commitment; its PDA is the one-human-one-identity dedup guard.
#[account]
#[derive(InitSpace)]
pub struct EnrollmentRecord {
    pub commitment: [u8; 32],
    pub leaf_index: u32,
    /// Wallet that enrolled. verify_personhood checks the signer against it,
    /// otherwise any signer could present someone else's public commitment + proof.
    pub wallet: Pubkey,
    pub is_frozen: bool,
    pub bump: u8,
    /// Unix timestamp of enrollment.
    pub enrolled_at: i64,
    /// The verifier credential account this enrollment was derived from (audit trail).
    pub credential: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct NullifierRecord {
    pub created_at: i64,
}
