use nix::fcntl::open;
use nix::fcntl::OFlag;
use nix::libc::c_void;
use nix::libc::write;
use nix::sys::stat::Mode;
use std::{thread, time};

use std::path::Path;
use std::thread::Thread;
fn main() {
    println!("open remoteproc");
    let path = Path::new("/sys/class/remoteproc/remoteproc0/firmware");
    if path.exists() {
        println!("{:?} exists", path);
        let fd = open(path, OFlag::O_RDWR | OFlag::O_SYNC, Mode::empty()).unwrap();
        let firmware_name = String::from("image_echo_test");
        let buf = firmware_name.into_bytes();
        unsafe {
            let size = write(fd, buf.as_ptr() as *const c_void, buf.len());
            println!("wrote {} bytes to remoteproc", size);
        }
        let state_path = Path::new("/sys/class/remoteproc/remoteproc0/state");
        let fd = open(state_path, OFlag::O_RDWR | OFlag::O_SYNC, Mode::empty()).unwrap();
        let command = String::from("start");
        let command_buf = command.into_bytes();
        unsafe {
            let size = write(fd, command_buf.as_ptr() as *const c_void, command_buf.len());
            println!("Wrote {} bytes to start remoteproc", size);
        }
        thread::sleep(time::Duration::from_millis(100));
        let stop_command = String::from("stop");
        let stop_command_buf = stop_command.into_bytes();
        unsafe {
            let size = write(
                fd,
                stop_command_buf.as_ptr() as *const c_void,
                stop_command_buf.len(),
            );
            println!("wrote {} bytes to stop remoteproc", size);
        }
    } else {
        panic!("Please check your system, you may not have remoteproc enabled");
    }
}
