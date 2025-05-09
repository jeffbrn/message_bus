use std::sync::{Condvar, Mutex, Arc};

/// holds the state of the signal
struct Flag {
    _state: bool, // true if signaled
}

pub struct Signal {
    status: Arc<Mutex<Flag>>,
    condvar: Arc<Condvar>,
}

#[cfg(test)]
impl Signal {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(Flag { _state: false })),
            condvar: Arc::new(Condvar::new()),
        }
    }

    /// wait for the signal to be set by another thread
    pub fn wait(&mut self) {
        println!("Signal waiting");
        let mut guard = self.status.lock().unwrap();
        while !guard._state {
            guard = self.condvar.wait(guard).unwrap();
        }
        guard._state = false;
        println!("Signal reset");
    }

    /// signal any waiting thread - only one waiting thread unblocks
    pub fn notify(&mut self) {
        println!("Signal notified");
        let mut guard = self.status.lock().unwrap();
        guard._state = true;
        self.condvar.notify_all();
    }
}

impl Clone for Signal {
    /// clone for sharing the signal across threads
    fn clone(&self) -> Self {
        Self {
            status: Arc::clone(&self.status),
            condvar: Arc::clone(&self.condvar),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::{self, sleep};
    use std::time::Duration;

    #[test]
    fn test_signal() {
        let mut signal = Signal::new();
        let mut signal_clone = signal.clone();

        let thrd = thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            signal_clone.notify();
            sleep(Duration::from_millis(500));
            signal_clone.notify();
        });

        signal.wait();
        {
            let guard = signal.status.lock().unwrap();
            assert_eq!(guard._state, false);
        }
        signal.wait();
        {
            let guard = signal.status.lock().unwrap();
            assert_eq!(guard._state, false);
        }
        thrd.join().unwrap();
    }

    #[test]
    fn test_signal_multiple() {
        let mut signal_trigger = Signal::new();
        let mut signal_wait1 = signal_trigger.clone();
        let mut signal_wait2 = signal_trigger.clone();

        let thrd1 = thread::spawn(move || {
            signal_wait1.wait();
            println!("Thread 1 woke-up");
        });
        let thrd2 = thread::spawn(move || {
            signal_wait2.wait();
            println!("Thread 2 woke-up");
        });

        thread::sleep(Duration::from_secs(1));
        println!("First trigger");
        signal_trigger.notify();
        thread::sleep(Duration::from_millis(100));
        println!("Second trigger");
        signal_trigger.notify();
        thrd1.join().unwrap();
        thrd2.join().unwrap();
    }
}
