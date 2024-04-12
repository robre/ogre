use ore::state::Hash;

use crate::config::MAX_MINERS;


#[derive(Debug)]
pub enum ThreadType {
    Ogre,
    Submitter,
    Stats,
    Oracle
}

#[derive(Debug)]
pub enum Activity {
    Idle,
    Premining,
    Mining,
    Accumulating(usize),
    Sending(usize),
}

#[derive(Debug)]
pub struct ThreadStatus {
    pub id: usize,
    pub ttype: ThreadType,
    pub activity: Activity,
}

#[derive(Debug)]
pub struct Stats {
    pub miners: u64,
    pub hashes_mined: u64,
    pub hashes_submitted: u64,
    pub ore_mined: u64,
    pub sol_spent: u64,
    pub lamports: u64,
    pub lifetime_hashes: u64,
    pub lifetime_rewards: u64,
    pub sent_sigs: u64,
    pub threads: Vec<ThreadStatus>,
    pub price: f64,
    pub oldprice: f64,
    pub supply: f64,
    pub oldsupply: f64,
    pub rate: u64,
    pub oldrate: u64,
    pub tip: u64,
    pub difficulty: Option<Hash>,
    pub notifications: Vec<String>,
}

impl Stats {
    pub fn can_make_more_miners(&self) -> bool {
        self.miners < MAX_MINERS as u64
    }
}
