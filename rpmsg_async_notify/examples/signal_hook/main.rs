use std::sync::mpsc::channel;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use bincode::deserialize;
use bincode::serialize_into;
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
use rpmsg_async_notify::remote_proc::RemoteprocManager;
use rpmsg_async_notify::Payload;
use rpmsg_async_notify::TimeStampTick;
use rpmsg_async_notify::PAYLOAD_MAX_SIZE;
use rpmsg_async_notify::{receive_tick, send_tick, TimeStamp, NUM_PAYLOADS};
use signal_hook::consts::SIGIO;
use signal_hook::iterator::Signals;

static mut endpoint_fd: Option<i32> = None;
fn main() {
    let remote_proc = RemoteprocManager::new("remoteproc0").unwrap();
    remote_proc
        .load_firmware("echo_test.elf".to_string())
        .unwrap();
    remote_proc.start().unwrap();

    let endpoint_path = prepare_environment();
    // register signal handler
    unsafe {
        endpoint_fd = if endpoint_path.exists() {
            Some(
                open(
                    &endpoint_path,
                    OFlag::O_RDWR | OFlag::O_NONBLOCK,
                    Mode::empty(),
                )
                .unwrap(),
            )
        } else {
            panic!("can't find endpoint in the system");
        };

        // register signal handler with signal-hook
        let mut signales = Signals::new(&[SIGIO]).unwrap();
        let handle = signales.handle();
        let (tx, rx) = channel();
        thread::spawn(move || {
            for sig in signales.forever() {
                //let time_stamp = ProcessTime::now();
                let time_stamp = clock();

                let mut receive_buf = [0u8; 1024];
                let bytes_rcvd = read(endpoint_fd.unwrap(), &mut receive_buf).unwrap();
                //let raw_pointer = receive_payload[..bytes_rcvd].as_ptr() as *const Payload;
                let r_payload: Payload = deserialize(&receive_buf).unwrap();
                //let message = TimeStamp {
                //id: r_payload.num,
                //time_stamp,
                //};
                let message = TimeStampTick {
                    id: r_payload.num,
                    time_stamp,
                };
                tx.send(message).unwrap();
            }
        });

        thread::spawn(move || loop {
            let message = rx.recv().unwrap();
            let mut r_tick = receive_tick.lock().unwrap();
            r_tick.insert(message.id as usize, message.time_stamp);
        });

        fcntl(endpoint_fd.unwrap(), F_SETOWN, getpid()); // Tell the kernel to whom to send the signal? Reflected by PID number
        let current_flags = fcntl(endpoint_fd.unwrap(), F_GETFL); // The application program reads the flag bit Oflags
        fcntl(endpoint_fd.unwrap(), F_SETFL, current_flags | O_ASYNC);

        // send a payload over
        for id in 0..NUM_PAYLOADS {
            let payload = Payload {
                num: id as u64,
                size: 20,
                data: vec![10; 20],
            };
            let mut sent_buf = [0u8; 1024];
            serialize_into(sent_buf.as_mut(), &payload).unwrap();

            let bytes_sent = write(endpoint_fd.unwrap(), &sent_buf[..PAYLOAD_MAX_SIZE]).unwrap();

            // clock time
            let mut tick_array = send_tick.lock().unwrap();
            //tick_array.insert(id, ProcessTime::now());

            let sent_t = clock();
            tick_array.insert(id, sent_t);

            //println!("sent out {} bytes", bytes_sent);
            sleep(Duration::from_micros(100));
        }
        sleep(Duration::from_secs(5));
        // calculate the average delay
        let r_tick = receive_tick.lock().unwrap();
        let s_tick = send_tick.lock().unwrap();
        let mut counter = 0;

        let mut total_diff = 0;
        let mut max_diff = clock_t::MIN;
        let mut min_diff = clock_t::MAX;
        //let mut total_diff = Duration::from_millis(0);
        //let mut max_diff = Duration::from_millis(u64::MIN);
        //let mut min_diff = Duration::from_millis(u64::MAX);

        for (id, receive_time) in r_tick.iter() {
            //println!("id: {}", id);
            if let Some(send_time) = s_tick.get(id) {
                //let diff = receive_time.duration_since(send_time.clone());
                let diff = receive_time - send_time;
                if diff > 1000000 {
                    print!(
                        "message: {}, receive_time: {}, send_time: {}, diff: {}",
                        id, receive_time, send_time, diff
                    );
                }

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
        //println!("average delay: {:?}", total_diff.div_f32(counter as f32));
        println!("average delay: {:?}", total_diff as f32 / counter as f32);

        remote_proc.stop().unwrap();
        handle.close();
    }
}
