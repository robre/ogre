use crossbeam_channel::{unbounded, Sender, Receiver};
use solana_client::nonblocking::rpc_client::RpcClient as RPC;
use std::{thread, sync::{Arc, Mutex}};

use crate::{
    ogrethread::OgreThread,
    submitterv2::SubmitterThread,
    miner::Miner,
    miner::MinerState,
    stats::Stats, config::{PRIO_FEE, RETRIES, MINERLIMIT},
};

pub struct Ogre {
    pub rpc: String,
    pub jito: String,
    pub stats: Arc<Mutex<Stats>>,
}

impl Ogre {
    pub async fn new(ogrethreads: u32, submitterthreads: u32, rpc: &str, jito: &str, miners: Vec<Miner>, stats: Arc<Mutex<Stats>>) -> Self {
        if submitterthreads < 1 {
            panic!()
        }
        // make each ogrethread and submitterthread
        let (loaded_sender, loaded_receiver) = unbounded::<Miner>();
        let (minable_sender, minable_receiver) = unbounded::<Miner>();

        for miner in miners {
            match miner.state {
                MinerState::New(_) | MinerState::Minable(_) => { 
                    // push to queue 1
                    minable_sender.send(miner).unwrap();
                },
                MinerState::Premined(_, _) | MinerState::Loaded(_, _)=> {
                    // push to queue 2
                    loaded_sender.send(miner).unwrap();
                },
            }
        }

        for i in 0..submitterthreads {
            // submitter sends to minable
            let sender = minable_sender.clone();
            // submitter receives from loaded
            let receiver = loaded_receiver.clone();
            let mut submitter = SubmitterThread::new(i.try_into().unwrap(), MINERLIMIT as usize, RETRIES, PRIO_FEE, sender, receiver, rpc.to_string().clone(), stats.clone());
            thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    submitter.start().await
                });
            });
        }

        for i in 0..ogrethreads {
            // ogre sends to loaded
            let sender = loaded_sender.clone();
            // ogre receives from minable
            let receiver = minable_receiver.clone();
            let mut ogre = OgreThread::new((submitterthreads + i).try_into().unwrap(), receiver, sender, rpc.to_string(), stats.clone()).await;
            thread::spawn(move || ogre.start());
        }

        Self {
            rpc: rpc.to_string(),
            jito: jito.to_string(),
            stats
        }

    }
}
