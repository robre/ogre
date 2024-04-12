use cached::proc_macro::cached;
use ore::{
    self,
    state::{Proof, Treasury},
    utils::AccountDeserialize,
    MINT_ADDRESS, PROOF, TREASURY_ADDRESS,
};

use oreprog::{anchor_lang::{ToAccountMetas, InstructionData}, IndexedSolution};
use solana_client::{nonblocking::rpc_client::RpcClient, client_error::Result, rpc_config::RpcSendTransactionConfig};
use solana_program::{pubkey::Pubkey, sysvar, instruction::Instruction, address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount}, message::{VersionedMessage, v0}};
use solana_sdk::{clock::Clock, commitment_config::{CommitmentConfig, CommitmentLevel}, account::Account, signature::Keypair, pubkey, transaction::VersionedTransaction};
use spl_associated_token_account::get_associated_token_address;

use crate::config::{MINT, ORE_COLLECTIVE};
pub async fn get_treasury(cluster: String) -> Treasury {
    let client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());
    let data = client
        .get_account_data(&TREASURY_ADDRESS)
        .await
        .expect("Failed to get treasury account");
    *Treasury::try_from_bytes(&data).expect("Failed to parse treasury account")
}

pub async fn get_proof(cluster: String, authority: Pubkey) -> Proof {
    let client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());
    let proof_address = proof_pubkey(authority);
    let data = client
        .get_account_data(&proof_address)
        .await
        .expect("Failed to get miner account");
    *Proof::try_from_bytes(&data).expect("Failed to parse miner account")
}

pub async fn get_account_data(cluster: String, account: Pubkey) -> Account{
    let client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());
    let data = client.get_account(&account).await.expect("failed to get miner account");
    data
}

pub async fn get_account_balance(cluster: String, account: Pubkey) -> u64 {
    let client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());
    let data = client.get_balance(&account).await.expect("failed to get miner account");
    data
}

pub async fn get_supply(cluster: String) -> f64 {
    let client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());
    let data = client.get_token_supply(&MINT).await.expect("failed to get token supply");
    data.ui_amount.unwrap()
}

pub async fn get_state(cluster: String, authority: &Pubkey) -> Option<Proof> {
    let client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());
    let proof_address = proof_pubkey(authority.clone());
    let data_opt = client
        .get_account_data(&proof_address)
        .await
        .ok();
    match data_opt {
        Some(data) => {
            Some(*Proof::try_from_bytes(&data).expect("Failed to parse miner account"))
        }
        None => None
    }
}

pub async fn get_clock_account(cluster: String) -> Clock {
    let client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());
    let data = client
        .get_account_data(&sysvar::clock::ID)
        .await
        .expect("Failed to get miner account");
    bincode::deserialize::<Clock>(&data).expect("Failed to deserialize clock")
}

pub fn pair_pubkey(k: &Keypair) -> Pubkey {
    let mut kb: [u8; 32] = [0 as u8;32];
    kb.copy_from_slice(&k.to_bytes()[32..]);
    Pubkey::new_from_array(kb)
}

#[cached]
pub fn proof_pubkey(authority: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[PROOF, authority.as_ref()], &ore::ID).0
}

pub fn miner_pubkey(authority: Pubkey, id: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"x", authority.as_ref(), &[id]], &ORE_COLLECTIVE)
}

#[cached]
pub fn treasury_tokens_pubkey() -> Pubkey {
    get_associated_token_address(&TREASURY_ADDRESS, &MINT_ADDRESS)
}

pub async fn create_tx_with_address_table_lookup(
    client: &RpcClient,
    instructions: &[Instruction],
    address_lookup_table_key: Pubkey,
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<(VersionedTransaction, RpcSendTransactionConfig)> {
    let raw_account = client.get_account(&address_lookup_table_key).await?;
    let address_lookup_table = AddressLookupTable::deserialize(&raw_account.data).unwrap();
    let address_lookup_table_account = AddressLookupTableAccount {
        key: address_lookup_table_key,
        addresses: address_lookup_table.addresses.to_vec(),
    };

    let (hash, slot) = client.get_latest_blockhash_with_commitment(CommitmentConfig::confirmed()).await.unwrap();
    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(v0::Message::try_compile(
            &pair_pubkey(&payer),
            instructions,
            &[address_lookup_table_account],
            hash,
        ).unwrap()),
        signers,
    )?;
    let send_cfg = RpcSendTransactionConfig {
        skip_preflight: true,
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
        max_retries: Some(1),
        min_context_slot: Some(slot),
    };

    assert!(tx.message.address_table_lookups().unwrap().len() > 0);
    Ok((tx, send_cfg))
}

pub fn register_ix(
    accounts: oreprog::accounts::Register,
    id: u8) -> Instruction {
    Instruction { 
        program_id: ORE_COLLECTIVE, 
        accounts: accounts.to_account_metas(Some(false)), 
        data: oreprog::instruction::Register{ id }.data() 
    }
}

pub fn mine_ix(
    accounts: oreprog::accounts::Mine,
    ids: Vec<IndexedSolution>) -> Instruction {
    Instruction { 
        program_id: ORE_COLLECTIVE, 
        accounts: accounts.to_account_metas(Some(false)), 
        data: oreprog::instruction::Mine{ ids }.data() 
    }
}

pub fn claim_ix(
    accounts: oreprog::accounts::Claim,
    amount: u64,
    id: u8) -> Instruction {
    Instruction { 
        program_id: ORE_COLLECTIVE, 
        accounts: accounts.to_account_metas(Some(false)), 
        data: oreprog::instruction::Claim{ amount, id }.data() 
    }
}
