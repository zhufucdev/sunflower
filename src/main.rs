mod ping;

use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel};
use std::thread;
use std::time::Duration;
use subprocess::{Popen, PopenConfig, Redirection};
use clap::Parser;
use crate::ping::{HttpPing, Ping, PingContext, StdoutPing};


#[derive(Parser)]
struct Cli {
    #[arg(default_value_t = 47990)]
    port: u16,
    #[arg(default_value = "localhost")]
    host: String,
}

fn main() {
    let args = Cli::parse();
    let (cancel_tx, cancel_rx) = channel();
    let cancel_rx_wrapped = Arc::new(Mutex::new(cancel_rx));
    ctrlc::set_handler(move || cancel_tx.send(()).expect("Failed to flush cancel signal")).unwrap();

    let stdout_ping = StdoutPing {};
    let http_ping = Arc::new(HttpPing { host: args.host.clone(), port: args.port });

    while cancel_rx_wrapped.lock().unwrap().try_recv().is_err() {
        let (ready_tx, ready_rx) = channel();
        let (fail_tx, fail_rx) = channel();

        let context = Arc::new(PingContext {
            process: Arc::new(Mutex::new(
                Popen::create(&["sunshine"], PopenConfig {
                    stdout: Redirection::Pipe,
                    stderr: Redirection::Pipe,
                    ..Default::default()
                })
                    .expect("Failed to start sunshine as subprocess")
            )),
            ready_tx,
            ready_rx: Arc::new(Mutex::new(ready_rx)),
            fail_tx,
            cancel_rx: cancel_rx_wrapped.clone(),
        });

        (|ctx: Arc<PingContext>| {
            thread::spawn(move || {
                stdout_ping.ping(ctx.clone());
            });
        })(context.clone());
        (|ctx: Arc<PingContext>| {
            http_ping.ping(ctx.clone());
        })(context.clone());
        
        fail_rx.recv().unwrap();
        cleanup(context.process.lock().unwrap().deref_mut());

        match context.process.lock().unwrap().wait() {
            Ok(_) => {
                if cancel_rx_wrapped.lock().unwrap().try_recv().is_ok() {}
            }
            Err(e) => println!("Error waiting for sunshine's termination: {e}")
        }

        eprintln!("Sunshine server failed unexpected. Restarting...");
        thread::sleep(Duration::from_secs(5));
    }
}

fn cleanup(p: &mut Popen) {
    match p.kill() {
        Ok(_) => {}
        Err(e) => eprintln!("Error while killing sunshine: {e}")
    }
}
