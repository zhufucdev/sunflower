use std::io::{BufRead, BufReader};
use std::sync::mpsc::channel;
use subprocess::{Popen, PopenConfig, Redirection};

fn cleanup(p: &mut Popen) {
    match p.kill() {
        Ok(_) => {}
        Err(e) => eprintln!("Error while killing sunshine: {e}")
    }
}

fn main() {
    let (tx, rx) = channel();
    ctrlc::set_handler(move || tx.send(()).expect("Failed to flush cancel signal")).unwrap();

    'l: while rx.try_recv().is_err() {
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
            if rx.try_recv().is_ok() {
                cleanup(&mut p);
                break 'l
            }
            if let Ok(l) = line {
                if l.contains("Unable to cleanup NvFBC") {
                    println!("Sunshine server failed. Restarting...");
                    cleanup(&mut p);
                }
            }
        }


        match p.wait() {
            Ok(_) => {
                if rx.try_recv().is_ok() {
                    cleanup(&mut p);
                    break 'l
                }
            }
            Err(e) => println!("Error waiting sunshine's determination: {e}")
        }

        println!("Sunshine server failed unexpected. Restarting...")
    }
}
