use std::clone::Clone;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use reqwest::StatusCode;
use subprocess::Popen;

#[derive(Clone)]
pub struct PingContext {
    pub process: Arc<Mutex<Popen>>,
    pub cancel_rx: Arc<Mutex<Receiver<()>>>,
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
        while context.cancel_rx.lock().unwrap().try_recv().is_err() {
            thread::sleep(Duration::from_secs(10));
            if let Ok(res) = reqwest::blocking::get(format!("{}:{}", self.host, self.port)) {
                if res.status() != StatusCode::OK {
                    context.fail_tx.send(()).unwrap();
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct StdoutPing {}

impl Ping for StdoutPing {
    fn ping(&self, context: Arc<PingContext>) {
        while context.cancel_rx.lock().unwrap().try_recv().is_err() {
            let err = context.process.lock().unwrap().stdout.take().expect("No std out");
            let reader = BufReader::new(err);

            let mut ready = false;

            for line in reader.lines() {
                if context.cancel_rx.lock().unwrap().try_recv().is_ok() {
                    context.fail_tx.send(()).unwrap();
                }
                if let Ok(l) = line {
                    if l.contains("Unable to cleanup NvFBC") {
                        if ready {
                            println!("Sunshine server failed. Restarting...");
                            context.fail_tx.send(()).unwrap();
                        }
                    }
                    if l.contains("Configuration UI available") && !ready {
                        ready = true;
                        context.ready_tx.send(()).unwrap();
                    }
                }
            }
        }
    }
}

