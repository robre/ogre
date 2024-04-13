use anchor_lang::prelude::*;
use anchor_lang::prelude::borsh::{BorshDeserialize};
use anchor_lang::solana_program::{
    program::invoke_signed,
    pubkey::Pubkey,
};
use anchor_spl::token::{self, Token, TokenAccount, Transfer as SplTransfer};

use crate::constants::*;

#[derive(Accounts)]
#[instruction(amount: u64, id: u8)]
pub struct Claim<'info> {
    /// Owner of this mine, Fee Payer
    #[account(mut)]
    authority: Signer<'info>,

    ///CHECK: ok
    #[account(mut, token::authority = authority)]
    beneficiary: Account<'info, TokenAccount>,
    ///CHECK: ok
    #[account(mut, seeds = [b"x", authority.key().as_ref(), &id.to_le_bytes()], bump)]
    miner: UncheckedAccount<'info>,

    ///CHECK: ok
    #[account(mut)]
    proof: UncheckedAccount<'info>,
    ///CHECK: ok
    #[account(mut)]
    treasury: UncheckedAccount<'info>,
    ///CHECK: ok
    #[account(mut)]
    treasury_tokens: UncheckedAccount<'info>,

    #[account(mut, address = MINER_COLLECTIVE_ORE_TREASURY)]
    miner_collective_ore_treasury: Account<'info, TokenAccount>,
    /// Ore Program
    ///CHECK: ok
    #[account(address = ORE_PROGRAM_ID, executable)]
    ore: UncheckedAccount<'info>,
    token_program: Program<'info, Token>,
}

impl Claim<'_> {
    pub fn handle(ctx: Context<Claim>, amount: u64, id: u8) -> Result<()> {
        let authority = &ctx.accounts.authority;
        let authority_key = authority.key();
        let beneficiary = &ctx.accounts.beneficiary;
        let proof = &ctx.accounts.proof;
        let treasury = &ctx.accounts.treasury;
        let treasury_tokens = &ctx.accounts.treasury_tokens;
        let token_program = &ctx.accounts.token_program;
        let miner = &ctx.accounts.miner;
        let miner_collective_ore_treasury = &ctx.accounts.miner_collective_ore_treasury;

        // 2% fee
        let fee = amount / 50;
        let remainder = amount - fee;

        let claim_ix = ore::instruction::claim(miner.key(), beneficiary.key(), remainder); 

        let signer: &[&[&[u8]]] = &[&[
            b"x", 
            authority_key.as_ref(), 
            &[id], 
            &[ctx.bumps.miner]
        ]];

        invoke_signed(
            &claim_ix, 
            &[
                miner.clone().to_account_info(),
                beneficiary.clone().to_account_info(),
                proof.to_account_info().clone(),
                treasury.to_account_info().clone(),
                treasury_tokens.to_account_info().clone(),
                token_program.to_account_info().clone(),
            ], 
            signer
        )?;

        let cpi_accounts = SplTransfer {
            from: beneficiary.to_account_info().clone(),
            to: miner_collective_ore_treasury.to_account_info().clone(),
            authority: authority.to_account_info().clone(),
        };

        let cpi_program = token_program.to_account_info().clone();
        token::transfer( 
            CpiContext::new(cpi_program, cpi_accounts),
            fee
        )?;


        Ok(())
    }
}
