
use bincode::deserialize;
use lazy_static::__Deref;
use nix::unistd::close;
use core::num;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::Mutex;
use cpu_time::ProcessTime;
use log::trace;
use nix::fcntl::open;
use nix::fcntl::OFlag;
use nix::libc::clock_t;
use nix::libc::fcntl;
use nix::libc::getpid;
use nix::libc::F_GETFL;
use nix::libc::F_SETFL;
use nix::libc::F_SETOWN;
use nix::libc::O_ASYNC;
use nix::sys::stat::Mode;
use nix::unistd::{read, write};
use rpmsg_async_notify::ffi::clock;
use rpmsg_async_notify::prepare_environment;
use rpmsg_async_notify::receive_tick_instant;
use rpmsg_async_notify::remote_proc::RemoteprocManager;
use rpmsg_async_notify::send_tick_instant;
use rpmsg_async_notify::Payload;
use rpmsg_async_notify::TimeStampTick;
use rpmsg_async_notify::PAYLOAD_MAX_SIZE;
use rpmsg_async_notify::{TimeStamp, NUM_PAYLOADS};
use signal_hook::consts::SIGIO;
use signal_hook::iterator::Signals;
use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

fn send_thread(){

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

    let remote_proc = RemoteprocManager::new("remoteproc0").unwrap();
    remote_proc
        .load_firmware_rs("echo_test.elf".to_string())
        .unwrap();
    remote_proc.start_rs().unwrap();

    let endpoint_path = prepare_environment();
    println!("endpoint path : {:?}", endpoint_path);
    // register signal handler
    unsafe {
        let fd = open(
            &endpoint_path,
            OFlag::O_RDWR | OFlag::O_NONBLOCK,
            Mode::empty(),
        )
        .unwrap();
        fcntl(fd, F_SETOWN, getpid()); // Tell the kernel to whom to send the signal? Reflected by PID number
        let current_flags = fcntl(fd, F_GETFL); // The application program reads the flag bit Oflags
        fcntl(fd, F_SETFL, current_flags | O_ASYNC);
    //let fd = OpenOptions::new().read(true).write(true).open(endpoint_path).unwrap();
    let endpoint_fd = Arc::new(Mutex::new(fd));
    let receive_tick = Arc::new(Mutex::new(HashMap::<usize, Instant>::new()));
    let send_tick = Arc::new(Mutex::new(HashMap::<usize, Instant>::new()));

    // register signal handler with signal-hook
    let (tx, rx) = channel::<TimeStamp>();

        // send thread
    let send_endpoint = endpoint_fd.clone();
    let s_tick = send_tick.clone();
    let r_tick = receive_tick.clone();
    let thread_handle = thread::spawn(move || 
        // send a payload over
        for id in 1..=num_payloads {
            let payload = Payload::new(id);
            let buf = payload.serialize();

            //send_endpoint.lock().unwrap().write_all(&buf).unwrap();
            let sent_bytes = write(*send_endpoint.lock().unwrap(), &buf).expect("failed to write to the endpoint");
            assert_eq!(buf.len(), sent_bytes);

            let time_stamp = Instant::now();
            let mut tick_array = s_tick.lock().unwrap();
            tick_array.insert(id, time_stamp);

            //check if send time increamented
            if let Some(ts) = tick_array.get(&(id - 1)){
                assert_eq!(true, ts < &time_stamp );
            }else{
                println!("can't find {}", id - 1);
            }

            println!("send {} payload, {} bytes", id, buf.len());
            if let Ok(message) = rx.recv() {
                let mut r = r_tick.lock().unwrap();
                // check if receive time is incremented
                if let Some(pre_ts) = r.get(&(message.id - 1)){
                    assert_eq!(true, pre_ts < &message.time_stamp);
                }else{
                    println!("can't find receive time for payload {}", message.id - 1);
                }
                r.insert(message.id, message.time_stamp);
            }
        }
    );

    // receive thread
    let mut signales = Signals::new(&[SIGIO]).unwrap();
    let handle = signales.handle();
    let receive_endpoint = endpoint_fd.clone();
    thread::spawn(move || {
        for sig in signales.forever() {
            println!("receive signal {}", sig);
            if sig == SIGIO{
                let time_stamp = Instant::now();
                let mut receive_buf = Vec::new();
                let received_bytes = read(*receive_endpoint.lock().unwrap(), &mut receive_buf).expect("can't read from endpoint");

                //let bytes_rcvd = receive_endpoint.lock().unwrap().read(&mut receive_buf).unwrap();
                println!("read out {} bytes", received_bytes);

                //let bytes_rcvd = read(*receive_endpoint.lock().unwrap().deref(), &mut receive_buf).unwrap();
                //let raw_pointer = receive_payload[..bytes_rcvd].as_ptr() as *const Payload;
                let r_payload: Payload = deserialize(&receive_buf[..received_bytes]).unwrap();

                let end_stamp = Instant::now();
                if end_stamp - time_stamp > Duration::from_micros(300) {
                    println!(
                        "send {:?} on read and serialize data",
                        end_stamp - time_stamp
                    );
                    println!("spend {:?} on handling signal", end_stamp - time_stamp);
                }
                let message = TimeStamp {
                    id: r_payload.num,
                    time_stamp,
                };
                tx.send(message).unwrap();
            }
        }
    });

    thread_handle.join().unwrap();

    // wait for all messages come back and processed
    sleep(Duration::from_secs(10));

    // calculate the average delay
    let r_tick = receive_tick.lock().unwrap();
    let s_tick = send_tick.lock().unwrap();
    let mut counter = 0;

    //let mut total_diff = 0;
    //let mut max_diff = clock_t::MIN;
    //let mut min_diff = clock_t::MAX;

    let mut total_diff = Duration::from_millis(0);
    let mut max_diff = Duration::from_millis(u64::MIN);
    let mut min_diff = Duration::from_millis(u64::MAX);

    let mut fd = File::create(format!("./signal_hook-{}.tsv", num_payloads)).unwrap();
    for (id, receive_time) in r_tick.iter() {
        //println!("id: {}", id);
        if let Some(send_time) = s_tick.get(id) {
            if receive_time < send_time {
                println!(
                    "[error]message: {}, receive_time: {:?}, send_time: {:?}",
                    id, receive_time, send_time
                );
                continue;
            }
            let diff = receive_time.duration_since(send_time.clone());
            let data_line = format!("{}\t{:.8}\n", id, diff.as_micros());
            fd.write_all(data_line.as_bytes()).unwrap();

            if diff > Duration::from_millis(1) {
                println!(
                    "message: {}, receive_time: {:?}, send_time: {:?}, diff: {:?}",
                    id, receive_time, send_time, diff
                );
            }
            //let diff = receive_time - send_time;

            if diff > max_diff {
                max_diff = diff;
            }
            if diff < min_diff {
                min_diff = diff;
            }
            total_diff += diff;
            counter += 1;
        }
    }
    println!("number of payload: {}", counter);
    println!("max delay: {:?}", max_diff);
    println!("min delay: {:?}", min_diff);
    println!("average delay: {:?}", total_diff.div_f32(counter as f32));
    //println!("average delay: {:?}", total_diff as f32 / counter as f32);

    remote_proc.stop().unwrap();
    handle.close();
    };
}
