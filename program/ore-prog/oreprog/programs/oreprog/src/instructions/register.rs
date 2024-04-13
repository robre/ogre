use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    program::invoke_signed,
};
use anchor_lang::system_program;
use crate::constants::*;
use crate::error::*;

use std::str::FromStr;

struct Ore;
impl anchor_lang::Id for Ore {
    fn id() -> Pubkey {
        Pubkey::from_str("mineRHF5r6S7HyD9SppBfVMXMavDkJsxwGesEvxZr2A").unwrap()
    }

}

#[derive(Accounts)]
#[instruction(id: u8)]
pub struct Register<'info> {
    
    ///CHECK: ok
    #[account(mut, seeds = [b"x", authority.key().as_ref(), &[id]], bump)]
    miner: UncheckedAccount<'info>,

    ///CHECK: ok
    #[account(mut)]
    proof: UncheckedAccount<'info>,

    #[account(mut)]
    authority: Signer<'info>,

    ///CHECK: ok
    #[account(mut, address = MINER_COLLECTIVE_TREASURY)]
    miner_collective_treasury: UncheckedAccount<'info>,

    ///CHECK: ok
    #[account(address = ORE_PROGRAM_ID, executable)]
    ore: Program<'info, Ore>,

    system_program: Program<'info, System>,
}

impl Register<'_> {
    pub fn handle(ctx: Context<Register>, id: u8) -> Result<()> {
        let register_ix = ore::instruction::register(ctx.accounts.miner.key());
        let authority = &ctx.accounts.authority.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"x", 
            authority.as_ref(), 
            &[id], 
            &[ctx.bumps.miner]
        ]];
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(), 
            system_program::Transfer {
                from: ctx.accounts.authority.to_account_info().clone(),
                to: ctx.accounts.miner.to_account_info().clone(),
        });

        // cover ore account rent 0.001_559_000 sol
        system_program::transfer(cpi_context, 2_600_000)?; 

        invoke_signed(
            &register_ix, 
            &[
                ctx.accounts.miner.to_account_info(),
                ctx.accounts.proof.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ], 
            signer_seeds)?;
        //return err!(OreCollectiveError::AnError);

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(), 
            system_program::Transfer {
                from: ctx.accounts.authority.to_account_info().clone(),
                to: ctx.accounts.miner_collective_treasury.to_account_info().clone(),
        });

        // .01 sol registration cost per account
        system_program::transfer(cpi_context, 10_000_000)?; 

        Ok(())
    }
}
