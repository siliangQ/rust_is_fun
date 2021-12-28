use std::thread::sleep;
use std::time::Duration;

use bincode::serialize_into;
use cpu_time::ProcessTime;
use log::trace;
use nix::fcntl::{open, OFlag};
use nix::libc::{fcntl, getpid, F_GETFL, F_SETFL, F_SETOWN, O_ASYNC};
use nix::sys::signal::{self, SigHandler};
use nix::sys::stat::Mode;
use nix::unistd::{read, write};
use rpmsg_async_notify::remote_proc::RemoteprocManager;
use rpmsg_async_notify::{
    prepare_environment, receive_tick, send_tick, Payload, NUM_PAYLOADS, PAYLOAD_MAX_SIZE,
};
static mut endpoint_fd: Option<i32> = None;

extern "C" fn sigio_handler(_: i32) {
    unsafe {
        if let Some(fd) = endpoint_fd {
            //let mut buf = receive_payload.lock().unwrap();
            let time_stamp = ProcessTime::now();
            let mut buf = [0u8; 1024];
            let bytes_rcvd = read(fd, buf.as_mut()).unwrap();
            let raw_pointer = buf.as_ptr() as *const Payload;
            //let r_payload: Payload = deserialize(&receive_payload).unwrap();
            {
                let mut r_tick = receive_tick.lock().unwrap();
                r_tick.insert((*raw_pointer).num as usize, time_stamp);
            }
            //println!("received {} bytes", bytes_rcvd);
            //println!("message : {:?}", (*raw_pointer).num);
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

        let sigaction = signal::SigAction::new(
            SigHandler::Handler(sigio_handler),
            signal::SaFlags::empty(),
            signal::SigSet::empty(),
        );
        signal::sigaction(signal::Signal::SIGIO, &sigaction).unwrap();

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
            tick_array.insert(id, ProcessTime::now());

            //println!("sent out {} bytes", bytes_sent);
            sleep(Duration::from_micros(100));
        }
        sleep(Duration::from_secs(5));
        // calculate the average delay
        let r_tick = receive_tick.lock().unwrap();
        let s_tick = send_tick.lock().unwrap();
        let mut total_diff = Duration::from_millis(0);
        let mut counter = 0;
        let mut max_diff = Duration::from_millis(u64::MIN);
        let mut min_diff = Duration::from_millis(u64::MAX);
        for (id, receive_time) in r_tick.iter() {
            //println!("id: {}", id);
            if let Some(send_time) = s_tick.get(id) {
                let diff = receive_time.duration_since(send_time.clone());
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
        remote_proc.stop().unwrap();
    }
}
