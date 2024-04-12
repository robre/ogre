use solana_program::{pubkey, pubkey::Pubkey};

/// Threads running the hashing/mining algorithm
pub const OGRETHREADS: u32 = 8;
/// Threads sending and confirming transactions
pub const SUBMITTERTHREADS: u32 = 3;

/// Keypair
pub const FUNDING: &str = "./kekXZMELSPFLDpUwMfWwvBbgujTun98eEBVZMrDwtFX.json";
/// Pubkey of above
pub const FUNDING_PK: Pubkey = pubkey!("kekXZMELSPFLDpUwMfWwvBbgujTun98eEBVZMrDwtFX");

/// RPC used for reading Data from the chain
pub const RPC: &str = "https://api.mainnet-beta.solana.com";
/// RPC used for sendTransaction only
pub const JITO: &str = "https://api.mainnet-beta.solana.com";
/// If sendTransaction RPC above is jito, set this to true to add a tip
pub const INCLUDE_TIP: bool = false;
/// Initial Tip amount. Will automatically increase/decrease depending on if your txs are landing
pub const TIP: u64 = 50_001;
/// Deprecated
pub const MAX_TIP: u64 = 500_000;
/// Jito account to tip to
pub const TIP_ACCOUNT: Pubkey = pubkey!("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49");
/// Ore Mint
pub const MINT: Pubkey = pubkey!("oreoN2tQbHXVaZsr3pf66A48miqcBXCDJozganhEJgz");
/// Address Lookup Table to use. You will need to set this up first!
pub const ALT: Pubkey = pubkey!("FQ8LwrRiuhNBfsZpKQdB8eMms59Vyh6VbzSNH7TVVjuW");

/// Compute Limits for Various Instructions. May vary - estimated upper bound
pub const CU_LIMIT_MINE: u32 = 2300 + 8200;
pub const CU_LIMIT_REGISTER: u32 = 7660 + 35_000;
pub const CU_LIMIT_TRANSFER: u32 = 5000;

/// Priority Fee
pub const PRIO_FEE: u64 = 500_099;


/// How long do we look for confirmations for sent transactions. During these congested times
/// should be > 60
pub const TIMEOUT: u64 = 120;

/// Deprecated
pub const MAX_MINERS: u32 = 50;
/// Number of Miners to Generate. Theoretically up to 255, but above 100 or so you'll need to use
/// two Address Lookup Tables
pub const MINER_COUNT: u8 = 100;
/// Number Of Miners Per Transaction. In theory this can go up to 50 or so, but then the
/// transactions don't get included as often. Seems like 20 works well.
pub const MINERLIMIT: u32 = 20;
/// Deprecated?
pub const RETRIES: u32 = 5;
/// Deprecated?
pub const SPAM: usize = 2;

pub const ORE_DECIMALS: u64 = 1000000000;
/// Hacky
pub const SOLPRICE: f64 = 174.0;

/// Accounts. Don't Change.
pub const ORE_COLLECTIVE: Pubkey = pubkey!("omcpZynsRS1Py8TP28zeTemamQoRPpuqwdqV8WXnL4M");
pub const ORE_TREASURY: Pubkey = pubkey!("FTap9fv2GPpWGqrLj3o4c9nHH7p36ih7NbSWHnrkQYqa");
pub const ORE_TREASURY_TOKENS: Pubkey = pubkey!("37ywg5kxKVb3q3bpvdYhQZBPHrHAXVo91RXoBBj7Boo9");
pub const ORE_COLLECTIVE_TREASURY: Pubkey = pubkey!("omc1vcb6CmMywXcDxyL77VaPYU98WyyaP3Mx6LBuaTr");
pub const ORE_COLLECTIVE_ORE_TREASURY: Pubkey = pubkey!("9idoAEtTrcnoXmrSYMx3pQQYiRLPND3NvcgJnfk6oihW");
