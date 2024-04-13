// we don't really need to check signer or pdas for mine instruction.
// let anyone send signatures for my miner!
// we can then check signatures in the claim instruction, e.g. make claim single account, and only
// claimable to signer 
//
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    program::invoke_signed,
    keccak::{hashv, Hash},
    pubkey::Pubkey
};

use ore::state::Proof;
use ore::utils::AccountDeserialize;
use crate::constants::*;


/// 10 bytes per solution
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct IndexedSolution {
    /// account id needed for pda
    pub id: u8,
    /// account bump
    pub bump: u8,
    /// solution nonce
    pub nonce: u64,
}

impl IndexedSolution {
    pub fn get_hash(&self, signer: Pubkey, current_hash: Hash) -> ore::state::Hash {
        hashv(&[
          current_hash.as_ref(),
          signer.as_ref(),
          self.nonce.to_le_bytes().as_slice(),
        ]).into()
    }
}


#[derive(Accounts)]
pub struct Mine<'info> {
    /// Owner of this mine, Fee Payer
    #[account(mut)]
    authority: Signer<'info>,
    /// Bus
    #[account(mut)]
    ///CHECK: ore checks this
    bus: UncheckedAccount<'info>,
    /// Treasury
    ///CHECK: ore checks this
    treasury: UncheckedAccount<'info>,
    /// Ore Program
    ///CHECK: ok
    #[account(address = ORE_PROGRAM_ID, executable)]
    ore: UncheckedAccount<'info>,
    //pub slot_hashes: Sysvar<'info, SlotHashes>,
    ///CHECK: ok
    pub slot_hashes: UncheckedAccount<'info>,

    // remaining accounts:
    //#[account( seeds = [b"x", authority.key().as_ref(), &id.to_le_bytes()], bump)]
    //miner: UncheckedAccount<'a>,
    //#[account(mut)]
    //proof: UncheckedAccount<'a>,
}

impl<'info> Mine<'_> {
    //pub fn handle<'c: 'info, 'info>(ctx: Context<'_, '_, 'c, 'info, Mine<'info>>, ids: Vec<IndexedSolution>) -> Result<()> {
    pub fn handle(ctx: Context<'_, '_, '_, 'info, Mine<'info>>, ids: Vec<IndexedSolution>) -> Result<()> {

        //let ore_program = &ctx.accounts.ore.to_account_info();
        let authority = &ctx.accounts.authority.key();
        let bus = &ctx.accounts.bus.clone();
        let treasury = &ctx.accounts.treasury.clone();
        let sh = &ctx.accounts.slot_hashes.clone();

        for (index, is) in ids.iter().enumerate() {
            let IndexedSolution { id, bump, nonce} = is;
            let miner = &ctx.remaining_accounts[index*2].clone();
            let proof = &ctx.remaining_accounts[index*2 + 1].clone();
            let proof_info = proof.to_account_info();
            // msg!("Miner: {:?}", &miner.key());
            // msg!("Proof: {:?}", &proof.key());
            let proof_data = proof_info.try_borrow_data()?;
            let proof_obj = Proof::try_from_bytes(&proof_data)?;
            let hash = is.get_hash(miner.key().clone(), proof_obj.hash.into());
            // msg!("{:?}", proof_obj.hash.to_string());
            // msg!("{:?}", nonce);
            // msg!("{:?}", hash.to_string());
            // register with ore, signing for miner
            let mine_ix = ore::instruction::mine(miner.key().clone(), ctx.accounts.bus.key().clone(), hash, *nonce); 
            // drop(proof_obj);
            drop(proof_data);
            // msg!("okkkk");
            let signer: &[&[&[u8]]] = &[&[
                b"x", 
                authority.as_ref(), 
                &[*id], 
                &[*bump]
            ]];

            invoke_signed(
                &mine_ix, 
                &[
                    miner.clone().to_account_info(),
                    bus.clone().to_account_info(),
                    proof_info.clone(),
                    treasury.to_account_info().clone(),
                    sh.to_account_info().clone(),
                ], 
                signer
            )?;
        }

        Ok(())
    }
}
