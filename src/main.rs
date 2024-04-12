use config::{TIP, MAX_MINERS, MINER_COUNT};
use glob::glob;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::read_keypair_file;
use stats::ThreadStatus;
use utils::{get_treasury, get_supply, miner_pubkey, pair_pubkey};
use std::{
    io::{
        stdout,
        Write
    },
    sync::{Arc, Mutex},
    time::{ Duration, Instant}, fs::File, task::Wake,
};
//use clap::{command, Parser, Subcommand};


use comfy_table::{Table, Cell, Color, modifiers::UTF8_SOLID_INNER_BORDERS, presets::UTF8_BORDERS_ONLY, Attribute};
use colored::Colorize;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod utils;
mod miner;
mod ogre;
mod ogrethread;
mod submitterv2;
mod config;
mod stats;

use crate::{
    miner::Miner, ogre::Ogre, miner::MinerState, config::{RPC, SUBMITTERTHREADS, OGRETHREADS, JITO, ORE_DECIMALS, FUNDING, SOLPRICE}, stats::Stats
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DataInnerInner {
    id: String,
    mintSymbol: String,
    vsToken: String,
    vsTokenSymbol: String,
    price: f64
}

#[derive(Deserialize)]
pub struct DataInner {
    oreoN2tQbHXVaZsr3pf66A48miqcBXCDJozganhEJgz: DataInnerInner
}

#[derive(Deserialize)]
pub struct Data {
    data: DataInner,
    timeTaken: f64
}

pub async fn loader(master_key: &str, count: u8, rpc: &str) -> Vec<Miner> {
    let mut miners: Vec<Miner> = vec![];
    let signer = read_keypair_file(master_key).unwrap();
    let signer_pk = pair_pubkey(&signer);
    for n in 0..count {
        let (miner_key, bump) = miner_pubkey(signer_pk, n);
        let miner = Miner::from_pubkey(&miner_key, n, bump, rpc).await;
        miners.push(miner);
    }
    println!("Loaded {} Miners", miners.len());
    miners
}

#[tokio::main]
async fn main() {
    let miners = loader(FUNDING, MINER_COUNT, RPC).await;
    let mut stdout = stdout();

    let lamports = miners.iter().fold(0 ,|r, s| r + s.lamports);
    let hashes = miners.iter().fold(0 ,|r, s| r + s.total_hashes);
    let rewards = miners.iter().fold(0 ,|r, s| r + s.total_rewards);

    let stats = Arc::new(Mutex::new(Stats { 
        miners: miners.len() as u64, 
        hashes_mined: 0,
        hashes_submitted: 0,
        ore_mined: 0,
        sol_spent: 0,
        lamports,
        lifetime_hashes: hashes,
        lifetime_rewards: rewards,
        sent_sigs: 0,
        threads: vec![],
        price: 0 as f64,
        oldprice: 0 as f64,
        supply: 0 as f64,
        oldsupply: 0 as f64,
        rate: 0,
        tip: TIP,
        oldrate: 0,
        difficulty: None,
        notifications: vec![],
    }));


    let ogre = Arc::new(Ogre::new(OGRETHREADS, SUBMITTERTHREADS, RPC, JITO, miners, stats.clone()).await);
    {
        // Price Bot
        let stats = stats.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                loop {
                    let treasury = get_treasury(RPC.to_string()).await;
                    let res = reqwest::get("https://price.jup.ag/v4/price?ids=oreoN2tQbHXVaZsr3pf66A48miqcBXCDJozganhEJgz").await.unwrap();
                    let data = res.json::<Data>().await.unwrap();

                    

                    //println!("{}", data.data.oreoN2tQbHXVaZsr3pf66A48miqcBXCDJozganhEJgz.price);
                    let supply = get_supply(RPC.to_string()).await;
                    {
                        let mut stats = stats.lock().unwrap();
                        stats.oldprice = stats.price;
                        stats.price = data.data.oreoN2tQbHXVaZsr3pf66A48miqcBXCDJozganhEJgz.price;
                        stats.oldsupply = stats.supply;
                        stats.supply = supply;
                        stats.oldrate = stats.rate;
                        stats.rate = treasury.reward_rate;
                        stats.difficulty = Some(treasury.difficulty);
                    }
                std::thread::sleep(Duration::from_millis(60 * 1000));
                }
            });
        });
    }
    
    {
        let stats = stats.clone();

        std::thread::spawn(move || {
            let start = Instant::now();
            let mut last_time = Instant::now();

            let mut table = Table::new();
            let mut threads = Table::new();
            let mut ore = Table::new();
            let mut profits = Table::new();


            //let mut log = File::create("stats.txt").unwrap();
            loop {
                std::thread::sleep(Duration::from_millis(10));

                table = Table::new();
                threads = Table::new();
                ore = Table::new();
                profits = Table::new();

                table.load_preset(UTF8_BORDERS_ONLY);
                table.set_content_arrangement(comfy_table::ContentArrangement::DynamicFullWidth);
                threads.load_preset(UTF8_BORDERS_ONLY);
                threads.set_content_arrangement(comfy_table::ContentArrangement::DynamicFullWidth);
                ore.load_preset(UTF8_BORDERS_ONLY);
                ore.set_content_arrangement(comfy_table::ContentArrangement::DynamicFullWidth);

                profits.load_preset(UTF8_BORDERS_ONLY);
                profits.set_content_arrangement(comfy_table::ContentArrangement::DynamicFullWidth);

                table.set_header(vec![
                                 Cell::new("Elapsed").add_attribute(Attribute::Bold), 
                                 Cell::new("Miners").add_attribute(Attribute::Bold), 
                                 Cell::new("H sub").add_attribute(Attribute::Bold), 
                                 Cell::new("H mined").add_attribute(Attribute::Bold), 
                                 Cell::new("HpS (sub)").add_attribute(Attribute::Bold), 
                                 Cell::new("HpS (mine)").add_attribute(Attribute::Bold), 
                                 Cell::new("Ore (session)").add_attribute(Attribute::Bold), 
                                 Cell::new("Fees (session)").add_attribute(Attribute::Bold), 
                                 Cell::new("Funds").add_attribute(Attribute::Bold), 
                                 Cell::new("H (life)").add_attribute(Attribute::Bold),
                                 Cell::new("Ore (life)").add_attribute(Attribute::Bold), 
                                 Cell::new("Tx Tries").add_attribute(Attribute::Bold)
                ]);
                threads.set_header(vec![
                                   Cell::new("type").add_attribute(Attribute::Bold), 
                                   Cell::new("id").add_attribute(Attribute::Bold), 
                                   Cell::new("status").add_attribute(Attribute::Bold)
                ]);
                ore.set_header(vec![
                               Cell::new("Reward Rate").add_attribute(Attribute::Bold), 
                               Cell::new("Supply").add_attribute(Attribute::Bold), 
                               Cell::new("MCAP").add_attribute(Attribute::Bold), 
                               Cell::new("Price").add_attribute(Attribute::Bold), 
                               Cell::new("Difficulty").add_attribute(Attribute::Bold), 
                               Cell::new("Premining Profit").add_attribute(Attribute::Bold),
                               Cell::new("Tip").add_attribute(Attribute::Bold)
                ]);
                profits.set_header(vec![
                               Cell::new("Profit").add_attribute(Attribute::Bold),
                               Cell::new("Session").add_attribute(Attribute::Bold),
                               Cell::new("Life").add_attribute(Attribute::Bold),
                               Cell::new("USD/Hr").add_attribute(Attribute::Bold),
                               Cell::new("USD/Day").add_attribute(Attribute::Bold),
                ]);

                let stats = stats.lock().unwrap();
                let elapsed = start.elapsed().as_secs_f64();
                if last_time.elapsed() >= Duration::from_millis(5000) {
                    let miners = stats.miners;
                    let hashes_mined = stats.hashes_mined;
                    let hashes_submitted = stats.hashes_submitted;
                    let ore_mined = stats.ore_mined;
                    let sol_spent = stats.sol_spent;
                    let lamports = stats.lamports;
                    let l_hashes = stats.lifetime_hashes;
                    let l_rewards = stats.lifetime_rewards;
                    let sent_sigs = stats.sent_sigs;
                    let vthreads = &stats.threads;
                    let price = stats.price;
                    let tip = stats.tip;
                    let oldprice = stats.oldprice;
                    let supply = stats.supply;
                    let oldsupply = stats.oldsupply;
                    let rate = stats.rate;
                    let oldrate = stats.oldrate;
                    let difficulty = stats.difficulty.unwrap();
                    let notifications = &stats.notifications;
                    stdout.write_all(b"\x1b[2J\x1b[3J\x1b[H").ok();
                    table.add_row(vec![
                                  Cell::new(format!("{:.4}s", elapsed)),
                                  Cell::new(format!("{:.4}", miners)),
                                  Cell::new(format!("{:.4}", hashes_submitted)),
                                  Cell::new(format!("{:.4}", hashes_mined)),
                                  Cell::new(format!("{:.4}", hashes_submitted as f64 / elapsed)),
                                  Cell::new(format!("{:.4}", hashes_mined as f64 / elapsed)),
                                  Cell::new(format!("{:.4}", ore_mined as f64 / ORE_DECIMALS as f64)),
                                  Cell::new(format!("{:.4}", sol_spent as f64 / LAMPORTS_PER_SOL as f64)),
                                  Cell::new(format!("{:.4}", lamports as f64 / LAMPORTS_PER_SOL as f64)),
                                  Cell::new(format!("{:.4}", l_hashes)),
                                  Cell::new(format!("{:.4}", l_rewards as f64 / ORE_DECIMALS as f64)),
                                  Cell::new(format!("{:.4}", sent_sigs)),
                    ]);
                    for thread in vthreads {
                        threads.add_row(vec![
                              Cell::new(format!("{:?}", thread.ttype)),
                              Cell::new(format!("{:?}", thread.id)),
                              Cell::new(format!("{:?}", thread.activity)),
                        ]);
                    }

                    let profit = price * rate as f64 / ORE_DECIMALS as f64 - 0.3;
                    let a = if profit > 0.0 {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let b = if rate > oldrate {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let c = if price > oldprice {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let d = if supply-oldsupply > 1.0 {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    ore.add_row(vec![
                          Cell::new(format!("{:.9} (${:.4})", rate as f64 / ORE_DECIMALS as f64, (rate as f64/ ORE_DECIMALS as f64) * price)).fg(b),
                          Cell::new(format!("{:.2} (+{:.2})", supply, supply-oldsupply)).fg(d),
                          Cell::new(format!("${:.2}", supply * price)).fg(c),
                          Cell::new(format!("${:.2}", price)).fg(c),
                          Cell::new(format!("{}", difficulty.to_string())),
                          Cell::new(format!("{:.4}", profit)).fg(a),
                          Cell::new(format!("{} (${:.4})", tip, SOLPRICE * tip as f64 / LAMPORTS_PER_SOL as f64)),
                    ]);

                    profits.add_row(vec![
                          Cell::new(format!("")),
                          Cell::new(format!("{:.4} (${:.4})", ore_mined as f64 / ORE_DECIMALS as f64, ore_mined as f64 * price/ ORE_DECIMALS as f64)),
                          Cell::new(format!("{:.4} (${:.4})", l_rewards as f64 / ORE_DECIMALS as f64,l_rewards as f64 * price / ORE_DECIMALS as f64)),
                          Cell::new(format!("${:.4}", (ore_mined as f64 * 3600.0  / (elapsed * ORE_DECIMALS as f64)) * price)),
                          Cell::new(format!("${:.4}", (ore_mined as f64 * 3600.0 * 24.0 / (elapsed * ORE_DECIMALS as f64)) * price)),
                    ]);

                    println!("{ore}");
                    println!("{table}");
                    println!("{profits}");
                    println!("{threads}");
                    for notification in notifications {
                        println!("{}", notification);
                    }

                    last_time = Instant::now();
                }
            }
        });
    }

    loop {
        std::thread::sleep(Duration::from_millis(5000));
    }
    //println!("End!");
}
