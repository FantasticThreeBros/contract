use anchor_lang::prelude::*;

use crate::{merkle, RegistryConfig, REGISTRY_SEED, TREE_DEPTH};

#[derive(Accounts)]
pub struct InitializeRegistry<'info> {
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

impl<'info> InitializeRegistry<'info> {
    pub fn initialize_registry(&mut self, bumps : &InitializeRegistryBumps) -> Result<()> {
    
        self.registry.set_inner(RegistryConfig {
            admin: self.admin.key(),
            is_frozen: false,
            // Root of an empty tree (NOT zeros): enroll_identity proves the next leaf is
            // empty against this root.
            merkle_root: merkle::empty_root(TREE_DEPTH),
            tree_depth: TREE_DEPTH,
            bump: bumps.registry,
            next_leaf_index: 0,
        });

        msg!("registry initialized. Admin: {}", self.registry.admin);
        Ok(())
    }
}
