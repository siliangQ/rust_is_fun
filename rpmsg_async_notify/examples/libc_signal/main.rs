use std::thread::sleep;
use std::time::{Duration, Instant};

use bincode::serialize_into;
use cpu_time::ProcessTime;
use log::trace;
use nix::fcntl::{open, OFlag};
use nix::libc::{fcntl, getpid, signal, F_GETFL, F_SETFL, F_SETOWN, O_ASYNC, SIGIO};
use nix::sys::stat::Mode;
use nix::unistd::{read, write};
use rpmsg_async_notify::ffi::clock;
use rpmsg_async_notify::remote_proc::RemoteprocManager;
use rpmsg_async_notify::{
    prepare_environment, receive_tick, send_tick, Payload, NUM_PAYLOADS, PAYLOAD_MAX_SIZE,
};
static mut endpoint_fd: Option<i32> = None;

pub fn sigio_handler(_: i32) {
    unsafe {
        if let Some(fd) = endpoint_fd {
            let time_stamp = clock();
            let mut buf = [0u8; 1024];
            let bytes_rcvd = read(fd, buf.as_mut()).unwrap();
            let raw_pointer = buf.as_ptr() as *const Payload;
            //let r_payload: Payload = deserialize(&receive_payload).unwrap();
            {
                let mut r_tick = receive_tick.lock().unwrap();
                r_tick.insert((*raw_pointer).num as usize, time_stamp);
            }
        }
    }
}
fn main() {
    let remote_proc = RemoteprocManager::new("remoteproc0").unwrap();
    remote_proc
        .load_firmware("echo_test.elf".to_string())
        .unwrap();
    remote_proc.start();

    let endpoint_path = prepare_environment();
    // register signal handler
    unsafe {
        endpoint_fd = if endpoint_path.exists() {
            trace!("opening endpoint handler");
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

        signal(SIGIO, sigio_handler as usize); // libc method to register a handler to a signal

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

            let mut tick_array = send_tick.lock().unwrap();
            tick_array.insert(id, clock());

            //println!("sent out {} bytes", bytes_sent);
            sleep(Duration::from_micros(100));
        }
        sleep(Duration::from_secs(1));

        // calculate the average delay
        let r_tick = receive_tick.lock().unwrap();
        let s_tick = send_tick.lock().unwrap();
        //println!("[debug] r_tick: {:?}", r_tick);
        //println!("[debug] s_tick: {:?}", s_tick);
        let mut counter = 0;

        //let mut total_diff = Duration::from_millis(0);
        //let mut max_diff = Duration::from_millis(u64::MIN);
        //let mut min_diff = Duration::from_millis(u64::MAX);
        let mut total_diff = 0;
        let mut max_diff = i64::MIN;
        let mut min_diff = i64::MAX;
        for (id, receive_time) in r_tick.iter() {
            //println!("id: {}", id);
            if let Some(send_time) = s_tick.get(id) {
                //println!(
                //"message: {}, receive_time:{:?}, send_time: {:?}",
                //id, receive_time, send_time
                //);
                //let diff = receive_time.duration_since(send_time.clone());
                let diff = receive_time - send_time;
                if diff < 0 {
                    continue;
                }

                if diff > 1000000 {
                    print!(
                        "message: {}, receive_time: {}, send_time: {}, diff: {}",
                        id, receive_time, send_time, diff
                    );
                }
                //println!("time diff: {:?}", diff);
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
    }
}
