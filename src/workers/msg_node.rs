use std::{sync::{atomic::{ AtomicBool, Ordering::Relaxed }, Arc}, thread, time::Duration};
use crossbeam_channel::{unbounded, Receiver};
use log::{debug, info};

use super::Worker;

pub struct MsgNode {
    thrd: Option<thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
    pub enabled: Arc<AtomicBool>
}

impl MsgNode {
    pub fn new(worker: Box<dyn Worker>, msg_in: Receiver<i32>) -> (Self, Receiver<i32>) {
        let (tx, rx) = unbounded();
        let flag = Arc::new(AtomicBool::new(true));
        let run = flag.clone();
        let enable = Arc::new(AtomicBool::new(true));
        let wrk_enable = enable.clone();
        let t = thread::spawn(move || {
            while flag.load(Relaxed) {
                let msg = msg_in.recv_timeout(Duration::from_millis(100)).ok();
                if msg.is_none() {
                    continue;
                }
                let msg = msg.unwrap();
                let enable = wrk_enable.load(Relaxed);
                if enable && worker.check_msg(msg) {
                    let result = worker.handle_msg(msg);
                    info!("Worker thread received message: {}, result = {}", msg, result);
                } else {
                    debug!("Passing message to next: {}", msg);
                    tx.send(msg).unwrap();
                }
            }
        });
        (Self { thrd: Some(t), running: run, enabled: enable }, rx)
    }
}

impl Drop for MsgNode {
    fn drop(&mut self) {
        self.running.store(false, Relaxed);
        if let Some(thrd) = self.thrd.take() {
            thrd.join().unwrap();
        }
    }
    
}
