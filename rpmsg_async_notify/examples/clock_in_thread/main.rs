use crossbeam::channel::unbounded;
use std::{
    collections::HashMap,
    env,
    sync::{mpsc::channel, Arc, Mutex},
    thread::{self, sleep},
    time::{Duration, Instant},
};
struct TMessage {
    id: usize,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let num_payloads = if args.len() > 1 {
        args[1]
            .parse::<usize>()
            .expect("please use a valid parameter")
    } else {
        1_000_000
    };
    let send_time_arc = Arc::new(Mutex::new(HashMap::new()));
    let receive_time_arc = Arc::new(Mutex::new(HashMap::new()));
    let (tx_o, rx_o) = unbounded::<TMessage>();
    let tx = tx_o.clone();
    let rx = rx_o.clone();
    let send_time = send_time_arc.clone();
    let send = thread::spawn(move || {
        for i in 0..num_payloads {
            send_time.lock().unwrap().insert(i, Instant::now());
            tx.send(TMessage { id: i }).unwrap();
            sleep(Duration::from_micros(30));
        }
    });
    let tx = tx_o.clone();
    let rx = rx_o.clone();
    let receive_time = receive_time_arc.clone();
    let receive = thread::spawn(move || loop {
        if let Ok(message) = rx.recv() {
            receive_time
                .lock()
                .unwrap()
                .insert(message.id, Instant::now());
        }
    });
    send.join().unwrap();

    //calculate the delay
    for (id, send_t) in send_time_arc.lock().unwrap().iter() {
        if let Some(receive_t) = receive_time_arc.lock().unwrap().get(id) {
            println!(
                "time_diff: {:?}, receive time: {:?}, send time: {:?}",
                receive_t.duration_since(*send_t),
                receive_t,
                send_t
            );
            if receive_t < send_t {
                println!("error happens");
                println!(
                    "time_diff: {:?}, receive time: {:?}, send time: {:?}",
                    receive_t.duration_since(*send_t),
                    receive_t,
                    send_t
                );
                break;
            }
        }
    }
}
