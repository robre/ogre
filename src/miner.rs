use std::{path::Path, thread::panicking, fmt::format, io};
use oreprog::constants::ORE_PROGRAM_ID;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::{Keypair, read_keypair_file, write_keypair_file}, system_transaction, commitment_config::CommitmentConfig};
use crate::{utils::{proof_pubkey, get_state, get_account_data, get_account_balance, claim_ix}, config::{RPC, JITO, ORE_TREASURY, ORE_COLLECTIVE_ORE_TREASURY, FUNDING_PK, ORE_TREASURY_TOKENS}};
use sha3::{Digest, Keccak256, digest::generic_array::GenericArray, digest::generic_array::typenum::U32};
//use solana_client::nonblocking::rpc_client::RpcClient as RPC;
use solana_sdk::{
    keccak::{hashv, Hash},
    signature::Signer

};
use solana_program::pubkey::Pubkey;

type Nonce = u64;

#[derive(Clone, Debug)]
pub enum MinerState {
    /// This Miner Keypair has been generated and no first solution exists yet. It is not
    /// registered.
    New(Hash),
    /// The first solution for this miner has been premined, but not submitted
    Premined(Nonce, Hash),
    /// This Miner can be Mined for a next solution.
    Minable(Hash),
    /// This Miner has a solution ready to be submitted 
    Loaded(Nonce, Hash),
}

#[derive(Clone, Debug)]
pub struct Miner {
    pub id: u8,
    pub bump: u8,
    pub pubkey: Pubkey,
    pub proof: Pubkey,
    pub state: MinerState,
    pub lamports: u64,
    pub total_rewards: u64,
    pub total_hashes: u64,
}

impl Miner {
    pub async fn from_pubkey(pubkey: &Pubkey, id: u8, bump: u8, rpc: &str) -> Self {
        let proof = get_state(rpc.to_string(), pubkey).await;
        let mut total_rewards: u64 = 0;
        let mut total_hashes: u64 = 0;
        let state = match proof {
            Some(p) => {
                total_rewards = p.total_rewards;
                total_hashes = p.total_hashes;
                MinerState::Minable(p.hash.into())
            }, 
            None => {
                let hash = hashv(&[
                    pubkey.to_bytes().as_slice(),
                ]);
                MinerState::New(hash)
            }
        };

        let acc = get_account_balance(rpc.to_string(), pubkey.clone()).await;
        println!("Loaded Miner {} : {} lamports | {} Hashes | {} Ore", pubkey.to_string(), acc, total_hashes, total_rewards);
        //println!("XYC \"{}\", \"{}\",", pubkey.to_string(), proof_pubkey(pubkey.clone()));

        Miner {
            id,
            bump,
            pubkey: pubkey.clone(),
            proof: proof_pubkey(pubkey.clone()),
            state,
            lamports:acc,
            total_rewards,
            total_hashes
        }
    }

    pub fn mine(&mut self, difficulty: Hash) {
        match self.state {
            MinerState::Premined(_, _) => {},
            MinerState::Loaded(_, _) => {},
            MinerState::New(hash) => {
                //println!("Mining New Account {}", self.pubkey);
                let mut next_hash: Hash;
                let mut nonce: u64 = 0;
                loop {
                    next_hash = hashv(&[
                          hash.to_bytes().as_slice(),
                          self.pubkey.to_bytes().as_slice(),
                          nonce.to_le_bytes().as_slice(),
                    ]);

                    if next_hash.le(&difficulty) {
                        break;
                    }
                    nonce += 1;
                }
                self.state = MinerState::Premined(nonce, next_hash);
                //println!("Found Nonce for Account {}: {}", self.pubkey, nonce);
            },
            MinerState::Minable(hash) => {
                //println!("Mining Minable Account {}", self.pubkey);
                let mut next_hash: Hash;
                let mut next_hash_gen: [u8; 32];
                let mut nonce: u64 = 0;
                let hash_bytes = hash.clone().to_bytes();
                let key_bytes = self.pubkey.to_bytes();
                let combined = [&hash_bytes[..], &key_bytes].concat();

                let mut hasher = Keccak256::default();
                loop {
                    // hasher.update(hash_bytes.as_slice());
                    // hasher.update(key_bytes.as_slice());
                    hasher.update(combined.as_slice());
                    hasher.update(nonce.to_le_bytes().as_slice());

                    //hasher.finalize_into_reset(&mut next_hash_gen);
                    next_hash = Hash(<[u8; 32]>::try_from(hasher.finalize_reset().as_slice()).unwrap());

                    // next_hash = hashv(&[
                    //       hash_bytes.as_slice(),
                    //       key_bytes.as_slice(),
                    //       nonce.to_le_bytes().as_slice(),
                    // ]);

                    if next_hash.le(&difficulty) {
                        break;
                    }
                    nonce += 1;
                    //hasher.reset();
                }
                //println!("{:?}", next_hash);
                self.state = MinerState::Loaded(nonce, next_hash);
                //println!("Found Nonce for Account {}: {}", self.pubkey, nonce);
            },
        }
    }

    /// send lamports to this account, if it has fewer than 1/2 of given amount
    pub fn fund(&mut self, payer: Keypair, amount: u64) -> Result<(), io::Error>{
        if self.lamports >= (amount/2) {
            return Ok(());
        }
        let diff = amount - self.lamports;

        let client = RpcClient::new_with_commitment(RPC, CommitmentConfig::confirmed());
        let jito = RpcClient::new_with_commitment(JITO, CommitmentConfig::confirmed());

        let (hash, slot) = client.get_latest_blockhash_with_commitment(CommitmentConfig::confirmed()).unwrap();
        let tx = system_transaction::transfer(&payer, &self.pubkey, diff, hash);
        let a = jito.send_transaction(&tx).unwrap();
        println!("Funding TX for {} {:?}", self.pubkey.to_string(), a);
        Ok(())
    }

    /// withdraw all ore from this account
    pub fn withdraw_ore(&mut self, receiver: Pubkey) -> Result<(), io::Error>{
        // let ix = claim_ix(
        //     oreprog::accounts::Claim{
        //         authority: FUNDING_PK,
        //         beneficiary: receiver,
        //         miner: self.pubkey,
        //         proof: self.proof,
        //         treasury: ORE_TREASURY,
        //         treasury_tokens: ORE_TREASURY_TOKENS,
        //         miner_collective_ore_treasury: ORE_COLLECTIVE_ORE_TREASURY,
        //         ore: ORE_PROGRAM_ID,
        //         token_program: spl_token::id(),
        //     }, self.total_rewards,  self.id);
        // // TODO:
        // let client = RpcClient::new_with_commitment(RPC, CommitmentConfig::confirmed());
        // let jito = RpcClient::new_with_commitment(JITO, CommitmentConfig::confirmed());

        // let (hash, slot) = client.get_latest_blockhash_with_commitment(CommitmentConfig::confirmed()).unwrap();

        // let (tx, send_cfg) = create_tx_with_address_table_lookup(&client, &[ix], ALT, &payer, &refkeys).await.unwrap();
        // let a = jito.send_transaction(&tx).unwrap();
        Ok(())
    }

    /// withdraw all lamports from this account
    pub fn withdraw_lamports(&mut self, receiver: Pubkey) -> Result<(), io::Error>{
        // TODO:
        Ok(())
    }

    /// withdraw all ore and lamports from this account
    pub fn empty(&mut self, receiver: Pubkey) -> Result<(), io::Error>{
        // TODO:
        Ok(())
    }
}
