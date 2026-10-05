use anchor_lang::prelude::*;

use crate::{error::PassportError, EnrollmentRecord, RegistryConfig, ENROLLMENT_SEED, REGISTRY_SEED};

/// Admin-only: freeze (or unfreeze) a single enrollment. A frozen identity can no
/// longer pass verify_personhood.
#[derive(Accounts)]
#[instruction(commitment: [u8; 32])]
pub struct FreezeIdentity<'info> {
    pub admin: Signer<'info>,

    #[account(seeds = [REGISTRY_SEED], bump = registry.bump, has_one = admin @ PassportError::Unauthorized)]
    pub registry: Account<'info, RegistryConfig>,

    #[account(mut, seeds = [ENROLLMENT_SEED, commitment.as_ref()], bump = enrollment.bump)]
    pub enrollment: Account<'info, EnrollmentRecord>,
}

impl<'info> FreezeIdentity<'info> {
    pub fn freeze_identity(&mut self, frozen: bool) -> Result<()> {
        self.enrollment.is_frozen = frozen;
        msg!("identity {} frozen = {}", self.enrollment.leaf_index, frozen);
        Ok(())
    }
}
