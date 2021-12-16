use crate::errors::ErrorCode;
use crate::structs::oracle::Oracle;
use crate::structs::pool::Pool;
use crate::structs::FeeTier;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_spl::token::Mint;

#[derive(Accounts)]
#[instruction(fee: u64, tick_spacing: u16)]

pub struct InitializeOracle<'info> {
    #[account(mut,
        seeds = [b"poolv1", fee_tier.key().as_ref(), token_x.key().as_ref(), token_y.key().as_ref()],
        bump = pool.load()?.bump
    )]
    pub pool: AccountLoader<'info, Pool>,
    #[account(
        seeds = [b"feetierv1", program_id.as_ref(), &fee.to_le_bytes(), &tick_spacing.to_le_bytes()],
        bump = fee_tier.load()?.bump
    )]
    pub fee_tier: AccountLoader<'info, FeeTier>,
    #[account(zero)]
    pub oracle: AccountLoader<'info, Oracle>,
    #[account(constraint = &token_x.key() == &pool.load()?.token_x,)]
    pub token_x: Account<'info, Mint>,
    #[account(constraint = &token_y.key() == &pool.load()?.token_y,)]
    pub token_y: Account<'info, Mint>,
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
}

pub fn handler(ctx: Context<InitializeOracle>) -> ProgramResult {
    msg!("INVARIANT: INITIALIZE ORACLE");

    let oracle = &mut ctx.accounts.oracle.load_init()?;
    let pool = &mut ctx.accounts.pool.load_mut()?;

    require!(
        pool.oracle_initialized == false,
        ErrorCode::OracleAlreadyInitialized
    );

    pool.set_oracle(ctx.accounts.oracle.key());
    oracle.init();

    Ok(())
}
