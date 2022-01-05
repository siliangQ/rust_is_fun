use bincode::deserialize;
use bincode::serialize_into;
use bincode::serialized_size;
use core::num;
use cpu_time::ProcessTime;
use lazy_static::__Deref;
use log::trace;
use nix::fcntl::open;
use nix::fcntl::OFlag;
use nix::libc;
use nix::libc::c_int;
use nix::libc::clock_t;
use nix::libc::epoll_event;
use nix::libc::fcntl;
use nix::libc::getpid;
use nix::libc::FD_CLOEXEC;
use nix::libc::F_GETFD;
use nix::libc::F_GETFL;
use nix::libc::F_SETFD;
use nix::libc::F_SETFL;
use nix::libc::F_SETOWN;
use nix::libc::O_ASYNC;
use nix::sys::stat::Mode;
use nix::unistd::close;
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
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::io::Write;
use std::ops::DerefMut;
use std::os::unix::prelude::RawFd;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

#[allow(unused_macros)]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { nix::libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

// create the epoll handler
fn epoll_create() -> io::Result<RawFd> {
    let fd = syscall!(epoll_create1(0))?;
    if let Ok(flags) = syscall!(fcntl(fd, F_GETFD)) {
        let _ = syscall!(fcntl(fd, F_SETFD, flags | FD_CLOEXEC))?;
    }
    Ok(fd)
}

// bind epoll handler's interest
fn add_interest(epoll_fd: RawFd, fd: RawFd, mut event: libc::epoll_event) -> io::Result<()> {
    syscall!(epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event))?;
    Ok(())
}

// modify epoll handler's interest
fn modify_interest(epoll_fd: RawFd, fd: RawFd, mut event: libc::epoll_event) -> io::Result<()> {
    syscall!(epoll_ctl(epoll_fd, libc::EPOLL_CTL_MOD, fd, &mut event))?;
    Ok(())
}

// remove the fd from epoll handler's interest
fn remove_interest(epoll_fd: RawFd, fd: RawFd) -> io::Result<()> {
    syscall!(epoll_ctl(
        epoll_fd,
        libc::EPOLL_CTL_DEL,
        fd,
        std::ptr::null_mut()
    ))?;
    Ok(())
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

        let epoll_fd = epoll_create().expect("can't create epoll handler");

        let event = epoll_event {
            events: (libc::EPOLLIN) as u32,
            u64: 0,
        };
        add_interest(epoll_fd, fd, event).expect("can't add rpmsg device to epoll's interests");

        let endpoint_fd = Arc::new(Mutex::new(fd));
        let receive_tick = Arc::new(Mutex::new(HashMap::<usize, Instant>::new()));
        let send_tick = Arc::new(Mutex::new(HashMap::<usize, Instant>::new()));

        // register signal handler with signal-hook
        let (tx, rx) = channel::<TimeStamp>();
        let mut events: Vec<epoll_event> = Vec::with_capacity(1024);

        let receive_endpoint = endpoint_fd.clone();
        let receive_thread = thread::spawn(move || loop {
            let res = match syscall!(epoll_wait(
                epoll_fd,
                events.as_mut_ptr() as *mut epoll_event,
                1024,
                -1
            )) {
                Ok(v) => v,
                Err(e) => panic!("epoll error during wait, error: {}", e),
            };
            events.set_len(res as usize);
            for event in events.iter() {
                if event.events == libc::EPOLLIN as u32 {
                    //println!("receive epoll event: {:?}", event);
                    let time_stamp = Instant::now();
                    let mut receive_buf = vec![10; 1024];
                    let received_bytes = read(*receive_endpoint.lock().unwrap(), &mut receive_buf)
                        .expect("can't read from endpoint");

                    let r_payload: Payload = deserialize(&receive_buf[..received_bytes]).unwrap();
                    println!("receive message: {:?}", r_payload);
                    // helper to investiagte the time distribution
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

        // send thread
        let send_endpoint = endpoint_fd.clone();
        let s_tick = send_tick.clone();
        let r_tick = receive_tick.clone();
        let thread_handle = thread::spawn(move || {
            // send a payload over
            for id in 1..=num_payloads {
                // construct the payload
                let payload = Payload::new(id);
                let mut sent_buf = [10u8; 1024];
                let ready_bytes = serialized_size(&payload).unwrap() as usize;
                serialize_into(sent_buf.as_mut(), &payload).unwrap();

                let time_stamp = Instant::now(); // start timing before the write function solve the problem
                                                 //send_endpoint.lock().unwrap().write_all(&buf).unwrap();

                let sent_bytes = write(*send_endpoint.lock().unwrap(), &sent_buf[..ready_bytes])
                    .expect("failed to write to the endpoint");

                //println!("send out message");
                let mut tick_array = s_tick.lock().unwrap();
                tick_array.insert(id, time_stamp);

                if let Ok(message) = rx.recv() {
                    let mut r = r_tick.lock().unwrap();
                    r.insert(message.id, message.time_stamp);
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

                if diff > Duration::from_millis(3) {
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
    };
}
