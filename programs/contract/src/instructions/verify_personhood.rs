use anchor_lang::{prelude::*, solana_program::instruction::get_stack_height};
use solana_instructions_sysvar::{load_current_index_checked, load_instruction_at_checked};

use crate::{
    error::PassportError,
    merkle::{self, Node},
    EnrollmentRecord, NullifierRecord, RegistryConfig, ENROLLMENT_SEED, NULLIFIER_SEED,
    REGISTRY_SEED,
};

/// Called via CPI from a consuming dApp, in the same transaction.
#[derive(Accounts)]
#[instruction(commitment: [u8; 32], consuming_program_id: Pubkey)]
pub struct VerifyPersonhood<'info> {
    /// Must be an explicit signer: identity is never inferred from the CPI context,
    /// otherwise any program could probe an arbitrary wallet's enrollment without consent.
    #[account(mut)]
    pub wallet: Signer<'info>,

    #[account(seeds = [REGISTRY_SEED], bump = registry.bump)]
    pub registry: Account<'info, RegistryConfig>,

    /// Binds `commitment` to `wallet` (and supplies the leaf index). Without it any
    /// signer could present someone else's public commitment and a valid proof.
    #[account(
        seeds = [ENROLLMENT_SEED, commitment.as_ref()],
        bump = enrollment.bump,
        constraint = enrollment.wallet == wallet.key() @ PassportError::WalletMismatch
    )]
    pub enrollment: Account<'info, EnrollmentRecord>,

    /// `init_if_needed` only to return our own `NullifierAlreadyUsed` error on replay.
    #[account(
        init_if_needed,
        payer = wallet,
        space = 8 + NullifierRecord::INIT_SPACE,
        seeds = [NULLIFIER_SEED, consuming_program_id.as_ref(), commitment.as_ref()],
        bump
    )]
    pub nullifier: Account<'info, NullifierRecord>,

    /// Used to learn the calling program's id.
    /// CHECK: address-constrained to the instructions sysvar.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> VerifyPersonhood<'info> {
    pub fn verify_personhood(
        &mut self,
        commitment: [u8; 32],
        consuming_program_id: Pubkey,
        merkle_proof: Vec<Node>,
    ) -> Result<()> {
        require!(!self.registry.is_frozen, PassportError::RegistryFrozen);
        require!(!self.enrollment.is_frozen, PassportError::IdentityFrozen);

        // Pin down who is calling: the top-level instruction's program is the consuming
        // dApp. Direct calls are refused.
        // LIMITATION: with nested CPI (dApp -> helper -> passport) this identifies the
        // top-level program, not the immediate caller.
        require!(get_stack_height() > 1, PassportError::NotCalledViaCpi);
        let ixs = self.instructions.to_account_info();
        let current = load_current_index_checked(&ixs)?;
        let top_level = load_instruction_at_checked(current as usize, &ixs)?;
        require_keys_eq!(top_level.program_id, consuming_program_id, PassportError::ConsumerMismatch);

        // Merkle proof against the stored root.
        merkle::check_proof_len(self.registry.tree_depth, &merkle_proof)?;
        let root = merkle::compute_root(commitment, self.enrollment.leaf_index, &merkle_proof);
        require!(root == self.registry.merkle_root, PassportError::InvalidMerkleProof);

        // Replay guard.
        require!(self.nullifier.created_at == 0, PassportError::NullifierAlreadyUsed);
        self.nullifier.created_at = Clock::get()?.unix_timestamp;

        Ok(())
    }
}
