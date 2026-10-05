use anchor_lang::prelude::*;

use crate::{error::PassportError, RegistryConfig, REGISTRY_SEED};

/// Admin-only kill switch: while frozen, enroll_identity and verify_personhood fail.
#[derive(Accounts)]
pub struct SetRegistryFrozen<'info> {
    pub admin: Signer<'info>,

    #[account(mut, seeds = [REGISTRY_SEED], bump = registry.bump, has_one = admin @ PassportError::Unauthorized)]
    pub registry: Account<'info, RegistryConfig>,
}

impl<'info> SetRegistryFrozen<'info> {
    pub fn set_registry_frozen(&mut self, frozen: bool) -> Result<()> {
        self.registry.is_frozen = frozen;
        Ok(())
    }
}
