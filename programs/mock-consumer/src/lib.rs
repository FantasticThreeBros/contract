//! Test-only consuming dApp. Its own logic would continue after the CPI returns.
use anchor_lang::prelude::*;
use contract::{
    cpi::{accounts::VerifyPersonhood, verify_personhood},
    program::Contract,
};

declare_id!("7ozUsANZ1BiQ6Fxmq6nzP7E5XS5dAcYGZcMiKwHSdqng");

#[program]
pub mod mock_consumer {
    use super::*;

    pub fn gated_action(
        ctx: Context<GatedAction>,
        commitment: [u8; 32],
        merkle_proof: Vec<[u8; 32]>,
    ) -> Result<()> {
        let accounts = VerifyPersonhood {
            wallet: ctx.accounts.wallet.to_account_info(),
            registry: ctx.accounts.registry.to_account_info(),
            enrollment: ctx.accounts.enrollment.to_account_info(),
            nullifier: ctx.accounts.nullifier.to_account_info(),
            instructions: ctx.accounts.instructions.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.passport_program.key(), accounts);
        verify_personhood(cpi_ctx, commitment, crate::ID, merkle_proof)?;
        msg!("gated action executed");
        Ok(())
    }
}

/// Everything is Unchecked: the passport program does the real validation, and
/// `wallet` is deliberately not a `Signer` here so tests can prove passport rejects it.
#[derive(Accounts)]
pub struct GatedAction<'info> {
    /// CHECK: forwarded to solana_passport
    #[account(mut)]
    pub wallet: UncheckedAccount<'info>,
    /// CHECK: forwarded to solana_passport
    pub registry: UncheckedAccount<'info>,
    /// CHECK: forwarded to solana_passport
    pub enrollment: UncheckedAccount<'info>,
    /// CHECK: forwarded to solana_passport
    #[account(mut)]
    pub nullifier: UncheckedAccount<'info>,
    /// CHECK: forwarded to solana_passport
    pub instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub passport_program: Program<'info, Contract>,
}
