use std::sync::Condvar;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering::Relaxed}, Mutex, RwLock};
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::{debug, trace};
use crate::workers::{Worker, msg_node::MsgNode};

pub type MsgBusHandler = fn(&i32) -> ();

pub struct MessageBusSeq {
    data: Arc<RwLock<MsgBusState>>,
    input_msg: Sender<i32>,
    thrd: Option<thread::JoinHandle<()>>,
    workers: Vec<MsgNode>,
}

struct MsgBusState {
    unhandled_msg: Receiver<i32>,
    unhandled_msg_handler: Option<MsgBusHandler>,
    running: AtomicBool,
    pub num_msgs: usize,
    pub num_unhandled: usize,
}

impl MessageBusSeq {

    pub fn new() -> Self {
        let (msg_tx, msg_rx) = unbounded();
        let mut retval = Self {
            data: Arc::new(RwLock::new(MsgBusState {
                unhandled_msg: msg_rx,
                unhandled_msg_handler: None,
                running: AtomicBool::new(true),
                num_msgs: 0,
                num_unhandled: 0,
            })),
            input_msg: msg_tx,
            thrd: None,
            workers: vec![],
        };
        let (thrd_signal, thrd) = MessageBusSeq::init_mb_daemon(retval.data.clone());
        retval.thrd = Some(thrd);
        {
            trace!("Waiting for worker thread to start");
            let (lck, cvar) = &*thrd_signal;
            let lck = lck.lock().unwrap();
            drop(cvar.wait(lck).unwrap());
            debug!("Worker thread started");
        }
        retval
    }

    /// publish a message onto the bus
    pub fn publish(&self, msg: i32) {
        // update the message count
        {
            let mut data = self.data.write().unwrap();
            data.num_msgs += 1;
        }
        trace!("Publishing message: {}", msg);
        // put message onto the bus
        self.input_msg.send(msg).unwrap();
    }

    /// set a handler to receive unprocessed messages
    pub fn set_unhandled(&mut self, handler: MsgBusHandler) {
        self.data.write().unwrap().unhandled_msg_handler = Some(handler);
        trace!("Unhandled message handler set");
    }

    pub fn add_worker(&mut self, worker: Box<dyn Worker>) {
        let mut data =self.data.write().unwrap();
        let (node, msg_out) = MsgNode::new(worker, data.unhandled_msg.clone());
        self.workers.push(node);
        data.unhandled_msg = msg_out;
    }

    pub fn len(&self) -> usize {
        self.workers.len()
    }

    pub fn set_enabled(&mut self, idx: usize, enabled: bool) {
        if idx < self.workers.len() {
            self.workers[idx].enabled.store(enabled, Relaxed);
        }
    }

    /// Initialize the message bus daemon thread
    fn init_mb_daemon(thrd_data: Arc<RwLock<MsgBusState>>) -> (Arc<(Mutex<()>, Condvar)>, thread::JoinHandle<()>) {
        // setup  way for the deaomon to signal the main thread that it is ready to recive messages
        let startup = Arc::new((Mutex::new(()), Condvar::new()));
        let signal = startup.clone();
        // spawn the daemon thread
        let t = thread::spawn(move || {
            trace!("MsgBus thread starting");
            {
                startup.1.notify_all();
            }
            // keep running until  the stop flag is set
            while thrd_data.read().unwrap().running.load(Relaxed) {
                let queue = thrd_data.read().unwrap().unhandled_msg.clone();
                // check for any unhandled messages
                let msg = queue.recv_timeout(Duration::from_millis(100)).ok();
                if msg.is_none() {
                    // no message, so check if we are still running
                    continue;
                }
                // process the unhandled message
                let msg = msg.unwrap();
                debug!("MsgBus thread received message: {}", msg);
                {
                    let mut data = thrd_data.write().unwrap();
                    // update the message count
                    data.num_unhandled += 1;
                }
                // check if we have a handler for the message
                let handler = thrd_data.read().unwrap().unhandled_msg_handler;
                if let Some(handler) = handler {
                    // call the handler
                    handler(&msg);
                }
            }
            debug!("MsgBus thread ending");
        });
        (signal, t)
    }

}