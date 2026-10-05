pub mod constants;
pub mod credential;
pub mod error;
pub mod instructions;
pub mod merkle;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("JAmQdpmj99JLsWK2pntXKmU8fz228W5dKva6VNqpkcSi");

#[program]
pub mod contract {
    use super::*;

    pub fn initialize_registry(ctx: Context<InitializeRegistry>) -> Result<()> {
        ctx.accounts.initialize_registry(&ctx.bumps)
    }

    pub fn register_verifier(ctx: Context<RegisterVerifier>, verifier_key: Pubkey, is_active: bool) -> Result<()> {
        ctx.accounts.register_verifier(verifier_key, is_active, &ctx.bumps)
    }

    pub fn enroll_identity(
        ctx: Context<EnrollIdentity>,
        commitment: [u8; 32],
        merkle_proof: Vec<[u8; 32]>,
    ) -> Result<()> {
        ctx.accounts.enroll_identity(commitment, merkle_proof, &ctx.bumps)
    }

    pub fn verify_personhood(
        ctx: Context<VerifyPersonhood>,
        commitment: [u8; 32],
        consuming_program_id: Pubkey,
        merkle_proof: Vec<[u8; 32]>,
    ) -> Result<()> {
        ctx.accounts.verify_personhood(commitment, consuming_program_id, merkle_proof)
    }

    pub fn freeze_identity(ctx: Context<FreezeIdentity>, commitment: [u8; 32], frozen: bool) -> Result<()> {
        let _ = commitment; // used for the enrollment PDA seed
        ctx.accounts.freeze_identity(frozen)
    }

    pub fn set_registry_frozen(ctx: Context<SetRegistryFrozen>, frozen: bool) -> Result<()> {
        ctx.accounts.set_registry_frozen(frozen)
    }
}
