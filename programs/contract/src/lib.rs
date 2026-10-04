pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("75Eb9EvhBEtrqDzEz9Wz7WrYyPi9ZN8sXD8dJ7PmMcPK");

#[program]
pub mod contract {
    use super::*;


    pub fn initialze_registry(ctx : Context<InitalizeRegistry>) -> Result<()>{
        ctx.accounts.initialize_registry(&ctx.bumps)
    }
 
}
