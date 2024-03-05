use std::clone::Clone;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct PingContext {
    pub stdout: Arc<Mutex<File>>,
    pub canceled: Arc<Mutex<bool>>,
    pub failed: Arc<Mutex<bool>>,
    pub fail_tx: Sender<()>,
    pub ready_rx: Arc<Mutex<Receiver<()>>>,
    pub ready_tx: Sender<()>,
}

pub trait Ping {
    fn ping(&self, context: Arc<PingContext>);
}

#[derive(Clone)]
pub struct HttpPing {
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl Ping for HttpPing {
    fn ping(&self, context: Arc<PingContext>) {
        context.ready_rx.lock().unwrap().recv().unwrap();
        
        loop {
            let cancelled = *context.canceled.lock().unwrap();
            let failed = *context.failed.lock().unwrap();
            if cancelled || failed { break }
            thread::sleep(Duration::from_secs(10));
            if let Ok(_) = reqwest::blocking::get(format!("{}:{}", self.host, self.port)) {
                continue;
            }
            context.fail_tx.send(()).unwrap();
        }
    }
}

#[derive(Copy, Clone)]
pub struct StdoutPing {}

impl Ping for StdoutPing {
    fn ping(&self, context: Arc<PingContext>) {
        if *context.canceled.lock().unwrap() {
            return;
        }
        
        let stdout = context.stdout.lock().unwrap();
        let reader = BufReader::new(stdout.deref());

        let mut ready = false;

        for line in reader.lines() {
            if *context.canceled.lock().unwrap() {
                break;
            }
            if let Ok(l) = line {
                if l.contains("Unable to cleanup NvFBC") {
                    if ready {
                        context.fail_tx.send(()).unwrap();
                        break;
                    }
                }
                if l.contains("Configuration UI available") && !ready {
                    ready = true;
                    context.ready_tx.send(()).unwrap();
                }
            }
        }

        context.fail_tx.send(()).unwrap();
    }
}

