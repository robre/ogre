pub use anchor_lang;
use anchor_lang::prelude::*;
use solana_program::sysvar::slot_hashes::SlotHashes;

pub mod constants;
pub mod error;
pub use instructions::*;

declare_id!("omcpZynsRS1Py8TP28zeTemamQoRPpuqwdqV8WXnL4M");

pub mod instructions;

#[program]
pub mod oreprog {
    use solana_program::sysvar::SysvarId;

    use super::*;

    pub fn register(ctx: Context<Register>, id: u8) -> Result<()> {
        Register::handle(ctx, id)
    }

    //pub fn mine(ctx: Context<Mine>, ids: Vec<IndexedSolution>) -> Result<()> {
    pub fn mine<'info>(ctx: Context<'_, '_, '_, 'info, Mine<'info>>, ids: Vec<IndexedSolution>) -> Result<()> {
        // msg!("{:?}", ctx.accounts.slot_hashes);
        // msg!("expected {:?}", SlotHashes::id());
        Mine::handle(ctx, ids)
    }

    pub fn claim(ctx: Context<Claim>, amount: u64, id: u8) -> Result<()> {
        Claim::handle(ctx, amount ,id)
    }
}

