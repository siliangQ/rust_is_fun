use rpmsg_async_notify::time_utils::unix_time;
use std::ops::Sub;
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

fn main() -> Result<(), &'static str> {
    let t00 = Instant::now();
    sleep(Duration::from_millis(1000));
    let t01 = Instant::now();
    sleep(Duration::from_millis(500));
    let t02 = Instant::now();
    sleep(Duration::from_micros(500));
    let t03 = Instant::now();
    sleep(Duration::from_micros(100));
    let t04 = Instant::now();

    println!(
        "{:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
        process::id(),
        t00,
        t01,
        t02,
        t03,
        t04
    );

    println!(
        "{:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
        process::id(),
        t00,
        t01.duration_since(t00),
        t02.duration_since(t01),
        t03.duration_since(t02),
        t04.duration_since(t03)
    );

    return Ok(());
}
