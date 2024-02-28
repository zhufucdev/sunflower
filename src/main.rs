use std::io::{BufRead, BufReader};
use subprocess::{Popen, PopenConfig, Redirection};

fn main() {
    let mut running = true;
    ctrlc::set_handler(move || {
        running = false;
    }).unwrap();

    while running {
        let mut p =
            Popen::create(&["sunshine"], PopenConfig {
                stdout: Redirection::Pipe,
                stderr: Redirection::Pipe,
                ..Default::default()
            })
                .expect("Failed to start sunshine as subprocess");

        let err = p.stderr.take().expect("No std error");
        let reader = BufReader::new(err);

        for line in reader.lines() {
            if let Ok(l) = line { 
                if l.contains("Unable to cleanup NvFBC") {
                    println!("Sunshine failed. Restarting...");
                    match p.kill() {
                        Ok(_) => continue,
                        Err(e) => eprintln!("Error while killing sunshine: {e}")
                    }
                }
            }
        }
        
        match p.wait() {
            Ok(_) => {}
            Err(e) => println!("Error waiting sunshine's determination: {e}")
        }

        println!("Sunshine server panicked. Restarting...")
    }
}
