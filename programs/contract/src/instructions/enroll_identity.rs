use anchor_lang::prelude::*;

use crate::{
    credential::{CredentialReader, ReclaimCredential},
    error::PassportError,
    merkle::{self, Node, EMPTY_LEAF},
    EnrollmentRecord, RegistryConfig, VerifierConfig, ENROLLMENT_SEED, REGISTRY_SEED,
    VERIFIER_CONFIG_SEED,
};

#[derive(Accounts)]
#[instruction(commitment: [u8; 32])]
pub struct EnrollIdentity<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,

    #[account(
        seeds = [VERIFIER_CONFIG_SEED, verifier_config.verifier_key.as_ref()],
        bump = verifier_config.bump,
        constraint = verifier_config.is_active @ PassportError::VerifierNotActive
    )]
    pub verifier_config: Account<'info, VerifierConfig>,

    /// The verifier's credential account. Read only, NOT a CPI.
    /// Step 1 (owner check) is enforced here, before anything is created: without it a
    /// fabricated account with the same data shape could pass.
    /// CHECK: owner is checked against the registered verifier program; data parsed via
    /// `CredentialReader`.
    #[account(
        constraint = credential_account.owner == &verifier_config.verifier_key
            @ PassportError::InvalidCredentialOwner
    )]
    pub credential_account: UncheckedAccount<'info>,

    #[account(mut, seeds = [REGISTRY_SEED], bump = registry.bump)]
    pub registry: Account<'info, RegistryConfig>,

    /// `init_if_needed` so a duplicate yields our custom `AlreadyEnrolled` instead of the
    /// system program's generic "account already in use"; the handler rejects any
    /// pre-existing record. The `commitment` arg is only here because the IDL builder can't
    /// resolve a hash expression in seeds; the handler requires it to equal
    /// sha256(credential key), so it is fully determined by the credential.
    #[account(
        init_if_needed,
        payer = wallet,
        space = 8 + EnrollmentRecord::INIT_SPACE,
        seeds = [ENROLLMENT_SEED, commitment.as_ref()],
        bump
    )]
    pub enrollment: Account<'info, EnrollmentRecord>,

    pub system_program: Program<'info, System>,
}

impl<'info> EnrollIdentity<'info> {
    /// `merkle_proof`: sibling hashes (bottom-up) along the path of the next free leaf
    /// (`registry.next_leaf_index`), from the client's off-chain copy of the tree.
    ///
    /// POC SIMPLIFICATION: the full tree lives off-chain; only the root is stored. Two
    /// concurrent enrolls invalidate each other's proofs (client re-fetches and retries).
    /// A production design would use a concurrent Merkle tree (SPL account-compression).
    pub fn enroll_identity(
        &mut self,
        commitment: [u8; 32],
        merkle_proof: Vec<Node>,
        bumps: &EnrollIdentityBumps,
    ) -> Result<()> {
        require!(!self.registry.is_frozen, PassportError::RegistryFrozen);

        // Step 1 happened in the accounts constraint. Parse the layout (stubbed).
        let parsed = ReclaimCredential::parse(&self.credential_account.try_borrow_data()?)?;
        if let Some(subject) = parsed.subject() {
            require_keys_eq!(subject, self.wallet.key(), PassportError::WalletMismatch);
        }

        // Step 2: deterministic commitment, derived from the credential.
        require!(
            commitment == merkle::commitment_from_credential(&self.credential_account.key()),
            PassportError::InvalidCommitment
        );

        // Step 3: dedup guard.
        require!(self.enrollment.commitment == [0u8; 32], PassportError::AlreadyEnrolled);

        // Step 4: prove prior state, then recompute the root with the new leaf.
        let registry = &mut self.registry;
        merkle::check_proof_len(registry.tree_depth, &merkle_proof)?;
        let index = registry.next_leaf_index;
        require!((index as u64) < (1u64 << registry.tree_depth), PassportError::TreeFull);

        let old_root = merkle::compute_root(EMPTY_LEAF, index, &merkle_proof);
        require!(old_root == registry.merkle_root, PassportError::InvalidMerkleProof);
        let new_root = merkle::compute_root(commitment, index, &merkle_proof);

        self.enrollment.set_inner(EnrollmentRecord {
            commitment,
            leaf_index: index,
            wallet: self.wallet.key(),
            is_frozen: false,
            bump: bumps.enrollment,
            enrolled_at: Clock::get()?.unix_timestamp,
            credential: self.credential_account.key(),
        });

        // Step 5.
        registry.merkle_root = new_root;
        registry.next_leaf_index = index + 1;

        msg!("Enrolled leaf {}", index);
        Ok(())
    }
}
