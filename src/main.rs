mod ping;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel};
use std::thread;
use subprocess::{Popen, PopenConfig, Redirection};
use clap::Parser;
use crate::ping::{HttpPing, Ping, PingContext, StdoutPing};


#[derive(Parser)]
struct Cli {
    #[arg(default_value = "http://localhost")]
    host: String,
    #[arg(default_value_t = 47990)]
    port: u16,
}

fn main() {
    let args = Cli::parse();
    let canceled = Arc::new(Mutex::new(false));
    ctrlc::set_handler((|a: Arc<Mutex<bool>>| { move || *a.lock().unwrap() = true })(canceled.clone())).unwrap();

    let stdout_ping = StdoutPing {};
    let http_ping = Arc::new(HttpPing { host: args.host.clone(), port: args.port });

    loop {
        if *canceled.lock().unwrap() { break }
        
        let (ready_tx, ready_rx) = channel();
        let (fail_tx, fail_rx) = channel();

        let mut process = Popen::create(&["sunshine"], PopenConfig {
            stdout: Redirection::Pipe,
            stderr: Redirection::Pipe,
            ..Default::default()
        })
            .expect("Failed to start sunshine as subprocess");

        let context = Arc::new(PingContext {
            stdout: Arc::new(Mutex::new(process.stdout.take().unwrap())),
            ready_tx,
            ready_rx: Arc::new(Mutex::new(ready_rx)),
            failed: Arc::new(Mutex::new(false)),
            fail_tx,
            canceled: canceled.clone(),
        });

        let handles = [
            (|ctx: Arc<PingContext>| {
                thread::spawn(move || {
                    stdout_ping.ping(ctx.clone());
                })
            })(context.clone()),
            (|ctx: Arc<PingContext>, ping: Arc<HttpPing>| {
                thread::spawn(move || {
                    ping.ping(ctx.clone());
                })
            })(context.clone(), http_ping.clone())
        ];

        fail_rx.recv().unwrap();
        *context.failed.lock().unwrap() = true;
        cleanup(&mut process);

        if let Err(e) = process.wait() {
            println!("Error waiting for sunshine's termination: {e}")
        }

        if !*canceled.lock().unwrap() {
            println!("Sunshine server failed. Restarting...");
        } else {
            println!("Waiting for ping threads to exit");
        }
        for handle in handles {
            handle.join().unwrap();
        }
    }
}

fn cleanup(p: &mut Popen) {
    match p.kill() {
        Ok(_) => {}
        Err(e) => eprintln!("Error while killing sunshine: {e}")
    }
}