use core::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ore::utils::AccountDeserialize;
use crossbeam_channel::{unbounded, TryRecvError, Sender, Receiver};
use ore::state::Bus;
use ore::{self, BUS_COUNT, BUS_ADDRESSES, BUS_EPOCH_REWARDS};
use oreprog::anchor_lang::InstructionData;
use oreprog::constants::{MINER_COLLECTIVE_TREASURY, ORE_PROGRAM_ID};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_program::instruction::{Instruction, AccountMeta};
use solana_program::message::VersionedMessage;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::slot_hashes;
use solana_program::{system_instruction, address_lookup_table, system_program};
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::signature::{read_keypair_file, Signature, Keypair};
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
};

use solana_client::{client_error::{ClientError, ClientErrorKind, Result}};
use solana_transaction_status::TransactionConfirmationStatus;
use crate::config::{CU_LIMIT_MINE, CU_LIMIT_REGISTER, SPAM, FUNDING, CU_LIMIT_TRANSFER, JITO, TIP_ACCOUNT, TIP, ORE_DECIMALS, TIMEOUT, INCLUDE_TIP, RPC, PRIO_FEE, ALT, FUNDING_PK, ORE_TREASURY};
use crate::stats::{ThreadStatus, ThreadType, Activity};
use crate::utils::{get_treasury, pair_pubkey, create_tx_with_address_table_lookup, register_ix, mine_ix};
use crate::{
    miner::{Miner, MinerState}, utils::get_proof, stats::Stats
};
use rand::Rng;

use oreprog::{IndexedSolution};

/// csv mining record
struct Record {
    signature: String,
    timestamp: u64,
    accounts: u64
}


#[derive(Clone)]
pub struct SigTime {
    sig: Signature,
    time: Instant,
}

impl SigTime {
    pub fn new(sig: Signature) -> Self {
        Self { sig, time: Instant::now() }
    }
    pub fn expired(&self) -> bool {
        self.time.elapsed() > Duration::from_secs(TIMEOUT)
    }
}

impl fmt::Debug for SigTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SigTime").field("Sig", &&self.sig.to_string()[..6]).finish()
    }

}


pub struct Combo {
    miners: Vec<Miner>,
    sigs: Vec<SigTime>,
    ixs: Vec<Instruction>,
    stats: Arc<Mutex<Stats>>,
    has_account_creation: bool,
    has_tip: bool,
    tip_amount: u64,
}

impl Combo {
    pub async fn new(miners: Vec<Miner>, stats: Arc<Mutex<Stats>>, bus: Bus) -> Self {
        // generate CU
        let mut total_cu: u32 = 300; // compute budget progs?
        let mut ixs: Vec<Instruction> = vec![];
        let mut has_account_creation = false;
        let mut has_tip = false;
        let mut tip_amount = 0;

        let mut ids: Vec<IndexedSolution> = vec![];
        let mut mines: Vec<AccountMeta> = vec![];

        for miner in &miners {
            match miner.state {
                MinerState::New(_) | MinerState::Minable(_) => {
                    panic!("nope");
                },
                MinerState::Premined(nonce, _hash) => {
                    //let transfer_ix = system_instruction::transfer(&funding_pk, &miner.pubkey, 10_000_000 ); // 0.01 SOL

                    let ix_register = register_ix(oreprog::accounts::Register{
                        miner: miner.pubkey, 
                        proof: miner.proof,
                        authority: FUNDING_PK,
                        miner_collective_treasury: MINER_COLLECTIVE_TREASURY,
                        ore: ORE_PROGRAM_ID,
                        system_program: system_program::id(),
                    }, miner.id);


                    total_cu += CU_LIMIT_REGISTER + CU_LIMIT_MINE ;
                    ixs.push(ix_register);
                    ids.push(IndexedSolution{
                        id: miner.id,
                        bump: miner.bump,
                        nonce,
                    });
                    mines.push(AccountMeta { pubkey: miner.pubkey, is_signer: false, is_writable: true });
                    mines.push(AccountMeta { pubkey: miner.proof, is_signer: false, is_writable: true });
                    has_account_creation = true;
                },
                MinerState::Loaded(nonce, _hash) => {
                    ids.push(IndexedSolution{
                        id: miner.id,
                        bump: miner.bump,
                        nonce,
                    });
                    mines.push(AccountMeta { pubkey: miner.pubkey, is_signer: false, is_writable: true });
                    mines.push(AccountMeta { pubkey: miner.proof, is_signer: false, is_writable: true });
                    //let ix_mine = ore::instruction::mine(miner.pubkey.clone(), BUS_ADDRESSES[bus.id as usize], hash.into(), nonce);
                    total_cu += CU_LIMIT_MINE;
                    //ixs.push(ix_mine);
                }
            }
        }

        if ids.len() > 0 {
            let mut ix_mine = mine_ix(oreprog::accounts::Mine{
                authority: FUNDING_PK,
                bus: BUS_ADDRESSES[bus.id as usize],
                treasury: ORE_TREASURY,
                ore: ORE_PROGRAM_ID,
                slot_hashes: slot_hashes::id(),
            }, ids);
            ix_mine.accounts.append(mines.as_mut());
            ixs.push(ix_mine);
        }


        if INCLUDE_TIP {
            let lockstats = stats.lock().unwrap();
            let tip_ix = system_instruction::transfer(&miners[0].pubkey, &TIP_ACCOUNT, lockstats.tip ); // jito tip
            ixs.push(tip_ix);
            has_tip = true;
            tip_amount = lockstats.tip;
            total_cu += CU_LIMIT_TRANSFER;
        }

        let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(total_cu);
        let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(PRIO_FEE);
        
        ixs.insert(0, cu_limit_ix);
        ixs.insert(1, cu_price_ix);

        Self {
            miners,
            sigs: vec![],
            ixs,
            stats,
            has_account_creation,
            has_tip,
            tip_amount
        }
    }

    pub fn adjust_tip(&mut self)  {
        // change the tip amount
        if self.has_tip {
            let stats = self.stats.lock().unwrap();
            let last_ix = self.ixs.pop().unwrap();
            let tip_ix = system_instruction::transfer(&self.miners[0].pubkey, &TIP_ACCOUNT, stats.tip ); // jito tip
            self.ixs.push(tip_ix);
        }  
    }

    pub async fn build_send_with_alt(&mut self) -> Result<()> {
        let client = RpcClient::new_with_commitment(RPC.to_string(), CommitmentConfig::confirmed());
        let jito = RpcClient::new_with_commitment(JITO.to_string().clone(), CommitmentConfig::confirmed());


        let keypair = read_keypair_file(FUNDING).unwrap();
        let payer = read_keypair_file(FUNDING).unwrap();
        let keypairs = vec![keypair];
        let refkeys = keypairs.iter().collect::<Vec<&Keypair>>();

        let (tx, send_cfg) = create_tx_with_address_table_lookup(&client, self.ixs.as_slice(), ALT, &payer, &refkeys).await.unwrap();

        match jito.send_transaction_with_config(&tx, send_cfg).await {
            Ok(sig) => {
                self.sigs.push(SigTime::new(sig));
                {
                    let mut stats = self.stats.lock().unwrap();
                    stats.sent_sigs += 1;
                }
            }
            Err(err) => {
                {
                    let mut stats = self.stats.lock().unwrap();
                    stats.sent_sigs += 1;
                    stats.notifications.push(format!("[Combo::build_send] {:?}", err));
                }
            }
        }
        Ok(())
    }

    pub async fn build_send(&mut self) -> Result<()> {
        let client = RpcClient::new_with_commitment(RPC.to_string(), CommitmentConfig::confirmed());
        let jito = RpcClient::new_with_commitment(JITO.to_string().clone(), CommitmentConfig::confirmed());


        let keypair = read_keypair_file(FUNDING).unwrap();
        let payer = read_keypair_file(FUNDING).unwrap();
        let keypairs = vec![keypair];
        let refkeys = keypairs.iter().collect::<Vec<&Keypair>>();

        let (hash, slot) = 
            match client.get_latest_blockhash_with_commitment(CommitmentConfig::confirmed()).await {
                Ok(a) => a,
                Err(err) => {
                    {
                        let mut stats = self.stats.lock().unwrap();
                        stats.sent_sigs += 1;
                        stats.notifications.push(format!("[Combo::build_send] {:?}", err));
                    }
                    return Ok(());
                }
            };
        let send_cfg = RpcSendTransactionConfig {
            skip_preflight: true,
            preflight_commitment: Some(CommitmentLevel::Confirmed),
            encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
            max_retries: Some(1),
            min_context_slot: Some(slot),
        };

        let mut tx = Transaction::new_with_payer(self.ixs.as_slice(), Some(&FUNDING_PK));
        tx.sign(&refkeys, hash);

        match jito.send_transaction_with_config(&tx, send_cfg).await {
            Ok(sig) => {
                self.sigs.push(SigTime::new(sig));
                {
                    let mut stats = self.stats.lock().unwrap();
                    stats.sent_sigs += 1;
                }
            }
            Err(err) => {
                {
                    let mut stats = self.stats.lock().unwrap();
                    stats.sent_sigs += 1;
                    stats.notifications.push(format!("[Combo::build_send] {:?}", err));
                }
            }
        }
        Ok(())
    }

    pub async fn confirm(&mut self) -> Option<Vec<Miner>> {
        // check confirmations by iterating the sigs, throwing out sigs that expired
        let client = RpcClient::new_with_commitment(RPC.to_string(), CommitmentConfig::confirmed());
        let mut i: usize = 0;

        self.sigs.retain(|a| {!a.expired()});
        match client.get_signature_statuses(&self.sigs.iter().map(|x| {x.sig}).collect::<Vec<Signature>>()).await {
            Ok(sig_statuses) => {
                if sig_statuses.value.len() == 0 {
                    {
                        let mut stats = self.stats.lock().unwrap();
                        stats.notifications.push(format!("[Combo::confirm] Empty Result"));
                    }
                    return None;
                }
                for sig_status in sig_statuses.value {
                    if let Some(ss) = sig_status.as_ref() {
                        if ss.confirmation_status.is_some() {
                            let cc = ss.confirmation_status.as_ref().unwrap();
                            match cc {
                                TransactionConfirmationStatus::Processed => {}
                                TransactionConfirmationStatus::Confirmed |
                                    TransactionConfirmationStatus::Finalized => {
                                        {
                                            let mut stats = self.stats.lock().unwrap();
                                            stats.hashes_submitted += self.miners.len() as u64;
                                            stats.lifetime_hashes += self.miners.len() as u64;
                                            stats.notifications.push(format!("[Combo::confirm] Confirmed {} https://solana.fm/tx/{}", self.sigs[i].sig, self.sigs[i].sig));
                                        }
                                        return Some(self.miners.clone())
                                    }
                            }

                        } 
                    } 
                    i += 1;
                }

            }
            Err(e) => {
                {
                    //TODO:
                    let mut stats = self.stats.lock().unwrap();
                    stats.notifications.push(format!("[Combo::confirm] {:?}", e));
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct Pending {
    miner: Miner,
    sigs: Vec<SigTime>,
    retries: usize,
    rpc: String,
    spam: usize,
    stats: Arc<Mutex<Stats>>,
}


/// SubmitterThread
pub struct SubmitterThread {
    pub id: usize,
    /// maximum number of miners to submit for simultaniously
    pub batchsize: usize,
    /// number of retries
    pub retries: u32,
    /// prio fee
    pub priority_fee: u64,
    /// the miners currently submitting for
    pub miners: Vec<Miner>,
    /// the queue from which to pull the next miner
    pub lq: Receiver<Miner>,
    /// the queue to which to push miners
    pub mq: Sender<Miner>,
    pub rpc: String,
    pub stats: Arc<Mutex<Stats>>,
    pub combo: Option<Combo>,
}

impl SubmitterThread {
    pub fn new(id: usize, batchsize: usize, retries: u32, priority_fee: u64, sender: Sender<Miner>, receiver: Receiver<Miner>, rpc: String, stats: Arc<Mutex<Stats>>) -> Self {
        {
            let mut stats = stats.lock().unwrap();
            stats.threads.push(
                ThreadStatus {
                    id,
                    ttype: ThreadType::Submitter,
                    activity: Activity::Idle
                });
        }
        SubmitterThread { id, batchsize, retries, priority_fee, miners: vec![], lq: receiver, mq: sender , rpc, stats, combo: None}
    }


    pub async fn start(&mut self) {
        loop {
            {
                let mut stats = self.stats.lock().unwrap();
                if self.miners.len() == 0 {
                    // stats.notifications.push(format!("Empty SubmitterThread."));
                    stats.threads[self.id].activity = Activity::Idle;
                } else if self.miners.len() == self.batchsize {
                    stats.threads[self.id].activity = Activity::Sending(self.miners.len());
                } else {
                    stats.threads[self.id].activity = Activity::Accumulating(self.miners.len());
                }
            }

            if self.miners.len() < self.batchsize {
                let message = self.lq.try_recv();
                match message {
                    Ok(miner) => {
                        self.miners.push(miner);
                    },
                    Err(TryRecvError::Empty) => { 
                        std::thread::sleep(Duration::from_millis(5000));
                    }
                    Err(TryRecvError::Disconnected) => {
                        panic!()
                    },
                };
                continue;
            }
            
            // we have now a full batch.

            // if combo exists, check confirmations
            match self.combo {
                Some(ref mut combo) => {
                    if let Some(_) = combo.confirm().await {
                        {
                            // reduce tip
                            let mut stats = self.stats.lock().unwrap();
                            stats.tip = stats.tip / 2;
                        }
                        self.combo = None;
                        let mut rewards_added: u64 = 0;
                        while let Some(mut m) = self.miners.pop() {
                            std::thread::sleep(Duration::from_millis(500));
                            let state = get_proof(self.rpc.clone(), m.pubkey).await;
                            m.total_hashes = state.total_hashes;
                            rewards_added += state.total_rewards - m.total_rewards;
                            m.total_rewards = state.total_rewards;
                            let new_hash = state.hash;

                            m.state = MinerState::Minable(new_hash.into());
                            self.mq.send(m).unwrap();
                        }
                        {
                            let mut stats = self.stats.lock().unwrap();
                            stats.ore_mined += rewards_added;
                            stats.lifetime_rewards += rewards_added;
                        }
                    } else {
                        {
                            // Combo didn't land; adjust tip
                            let mut stats = self.stats.lock().unwrap();
                            if stats.tip > 500_000 {
                                stats.tip = TIP;
                            } else {
                                if (stats.rate as f64/ ORE_DECIMALS as f64) * stats.price < (185.0 * stats.tip as f64 / LAMPORTS_PER_SOL as f64) {
                                    stats.tip = stats.tip / 2;
                                } else {
                                    stats.tip = stats.tip * 101 / 100;
                                }
                            }
                            combo.adjust_tip();
                        }
                        combo.build_send_with_alt().await;
                    }

                },
                None => {
                    // create new combo
                    let treasury = get_treasury(self.rpc.clone()).await;
                    let bus = self.find_bus_id(treasury.reward_rate).await;
                    let mut combo = Combo::new(self.miners.clone(), self.stats.clone(), bus).await;
                    combo.build_send_with_alt().await;
                    self.combo = Some(combo);

                }
            }
            std::thread::sleep(Duration::from_millis(2000));
        }
    }


    async fn find_bus_id(&self, reward_rate: u64) -> Bus {
        let mut rng = rand::thread_rng();
        loop {
            
            let bus_id = rng.gen_range(0..8);
            if let Ok(bus) = self.get_bus(bus_id).await {
                if bus.rewards.gt(&reward_rate.saturating_mul(self.batchsize as u64 * 4)) {
                    return bus;
                }
            }
        }


    }
    // pub async fn busses(&self) {
    //     let client =
    //         RpcClient::new_with_commitment(self.rpc.clone(), CommitmentConfig::confirmed());
    //     for address in BUS_ADDRESSES.iter() {
    //         let data = client.get_account_data(address).await.unwrap();
    //         match Bus::try_from_bytes(&data) {
    //             Ok(bus) => {
    //                 println!("Bus {}: {:} ORE", bus.id, bus.rewards);
    //             }
    //             Err(_) => {}
    //         }
    //     }
    // }

    pub async fn get_bus(&self, id: usize) -> Result<Bus> {
        let client =
            RpcClient::new_with_commitment(self.rpc.clone(), CommitmentConfig::confirmed());
        let data = client.get_account_data(&BUS_ADDRESSES[id]).await?;
        Ok(*Bus::try_from_bytes(&data).unwrap())
    }

}

