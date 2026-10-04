use anchor_lang::prelude::*;

use crate::{RegistryConfig, REGISTRY_SEED, TREE_DEPTH};

#[derive(Accounts)]
pub struct InitalizeRegistry<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(init,
        payer=admin,
        space = 8 +  RegistryConfig::INIT_SPACE,
        seeds =[ REGISTRY_SEED ],
       bump

    )]
    pub registry: Account<'info, RegistryConfig>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitalizeRegistry<'info> {
    pub fn initialize_registry(&mut self, bumps : &InitalizeRegistryBumps) -> Result<()> {
    
        self.registry.set_inner(RegistryConfig {
            admin: self.admin.key(),
            is_frozen: false,
            merkle_root: [0u8; 32] ,
            tree_depth: TREE_DEPTH,
            bump: bumps.registry,
        });

        msg!("registry initialized. Admin: {}", self.registry.admin);
        Ok(())
    }
}
