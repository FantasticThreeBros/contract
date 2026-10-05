use anchor_lang::prelude::*;

use crate::{
    error::PassportError, RegistryConfig, VerifierConfig, REGISTRY_SEED, VERIFIER_CONFIG_SEED,
};

#[derive(Accounts)]
#[instruction(verifier_key: Pubkey)]
pub struct RegisterVerifier<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(seeds = [REGISTRY_SEED], bump=registry.bump, has_one = admin @PassportError::Unauthorized)]
    pub registry: Account<'info, RegistryConfig>,

    #[account(init_if_needed,
        payer = admin, space = 8 + VerifierConfig::INIT_SPACE, seeds = [VERIFIER_CONFIG_SEED,verifier_key.as_ref()],bump)]
    pub verifier_config: Account<'info, VerifierConfig>,

    pub system_program: Program<'info, System>,
}

impl<'info> RegisterVerifier<'info> {
    pub fn register_verifier(&mut self, verifier_key: Pubkey, gatekeeper_network: Pubkey, is_active: bool, bumps : &RegisterVerifierBumps) -> Result<()> {
        self.verifier_config.set_inner(VerifierConfig {
            verifier_key: verifier_key,
            gatekeeper_network,
            is_active: is_active,
            bump: bumps.verifier_config,
        });

        msg!("verifier_config registered  verifier_key :  {} , is_active : {}",verifier_key,is_active);

        Ok(())
    }
}
