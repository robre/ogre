use std::{time::Duration, path::Path, sync::{Arc, Mutex}};

use crossbeam_channel::{TryRecvError, Sender, Receiver};
use solana_sdk::{
    keccak::Hash
};

use crate::{
    miner::Miner, utils::get_treasury, stats::{Stats, ThreadStatus, ThreadType, Activity},
};

/// OgreThread just mines the next hash offline. No need for rpc connection or anything.
pub struct OgreThread {
    pub id: usize,
    /// the queue from which to pull the next miner
    pub mq: Receiver<Miner>,
    /// the queue to which to push miners
    pub lq: Sender<Miner>,
    pub difficulty: Hash,
    pub stats: Arc<Mutex<Stats>>,
}
impl OgreThread {
    pub async fn new(id: usize, mq: Receiver<Miner>, lq: Sender<Miner>, rpc: String, stats: Arc<Mutex<Stats>>) -> Self {
        let treasury = get_treasury(rpc).await;
        {
            let mut stats = stats.lock().unwrap();
            stats.threads.push(
                ThreadStatus {
                    id,
                    ttype: ThreadType::Ogre,
                    activity: Activity::Idle
                });
        }
        OgreThread {
            id,
            mq,
            lq,
            difficulty: treasury.difficulty.into(),
            stats
        }
    }

    pub fn start(&mut self) {
        loop {
            let message = self.mq.try_recv();
            match message {
                Ok(mut miner) => {
                    {
                        let mut stats = self.stats.lock().unwrap();
                        stats.threads[self.id].activity = Activity::Mining;
                    }
                    miner.mine(self.difficulty);
                    self.lq.send(miner).unwrap();
                    {
                        let mut stats = self.stats.lock().unwrap();
                        stats.threads[self.id].activity = Activity::Idle;
                        stats.hashes_mined += 1;
                    }
                },
                Err(TryRecvError::Empty) => {
                    // queue is empty, nothing to do. let's see if we can create a new miner
                    std::thread::sleep(Duration::from_millis(1000));
                }
                Err(TryRecvError::Disconnected) => {},
            }
        }

    }
}
