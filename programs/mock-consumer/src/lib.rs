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

    /// TEST/DEMO ONLY: plays the role of the Civic gateway program by writing an account in
    /// Civic's exact `GatewayToken` borsh layout (owned by this program). Lets tests and the
    /// devnet demo enroll without a real Civic pass; register this program's id as the
    /// verifier to use it. Real deployments register Civic's gateway program instead.
    pub fn issue_demo_pass(
        ctx: Context<IssueDemoPass>,
        owner_wallet: Pubkey,
        network: Pubkey,
        state: u8, // 0 Active, 1 Frozen, 2 Revoked
        expire_time: Option<i64>,
    ) -> Result<()> {
        let mut d: Vec<u8> = vec![0, 0]; // version, parent_gateway_token = None
        d.extend_from_slice(owner_wallet.as_ref());
        d.push(0); // owner_identity = None
        d.extend_from_slice(network.as_ref());
        d.extend_from_slice(ctx.accounts.payer.key().as_ref()); // issuing_gatekeeper
        d.push(state);
        match expire_time {
            Some(t) => {
                d.push(1);
                d.extend_from_slice(&t.to_le_bytes());
            }
            None => d.push(0),
        }
        d.resize(109, 0); // Civic fixes the account size at 109 bytes
        ctx.accounts.pass.try_borrow_mut_data()?[..109].copy_from_slice(&d);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(owner_wallet: Pubkey, network: Pubkey)]
pub struct IssueDemoPass<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: raw account we fill with Civic's layout.
    #[account(
        init,
        payer = payer,
        space = 109,
        seeds = [b"demo_pass", owner_wallet.as_ref(), network.as_ref()],
        bump
    )]
    pub pass: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
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
