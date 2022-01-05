extern crate nix;
extern crate serde;
extern crate snafu;
#[macro_use]
extern crate lazy_static;
use crate::channel::{
    ioctl_create_endpt, register_rpmsg_driver_for_device, search_control_interface,
    search_endpoint_path_by_name, RPMsgEndpointInfo,
};
use bincode::deserialize;
use bincode::serialize_into;
use bincode::serialized_size;
use cpu_time::ProcessTime;
use log::trace;
use nix::libc::clock_t;
use nix::sys::signal::{self, SigHandler};
use nix::unistd::write;
use nix::{
    fcntl::{open, OFlag},
    libc::{__u32, access, fcntl, getpid, signal, F_GETFL, F_SETFL, F_SETOWN, O_ASYNC, SIGIO},
    sys::stat::Mode,
    unistd::{read, AccessFlags},
};
use remote_proc::RemoteprocManager;
use serde::{Deserialize, Serialize};
use signal_hook::consts::SIGIO as SIGIO_HOOK;
use signal_hook::iterator::Signals;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};
use std::{mem, path::Path};

pub mod channel;
pub mod remote_proc;
pub mod time_utils;

pub const RPMSG_HEADER_LEN: u32 = 16;
pub const MAX_RPMSG_BUFF_SIZE: u32 = (512 - RPMSG_HEADER_LEN);
pub const PAYLOAD_MAX_SIZE: usize = (MAX_RPMSG_BUFF_SIZE - 24) as usize;
pub const RPMSG_ADDR_ANY: __u32 = 0xffffffff;
pub const NUM_PAYLOADS: usize = 1_00000;
lazy_static! {
    pub static ref receive_payload: Mutex<[u8; 1024]> = Mutex::new([0u8; 1024]);
    pub static ref send_payload: Mutex<[u8; 1024]> = Mutex::new([0u8; 1024]);
    pub static ref send_tick: Mutex<HashMap<usize, clock_t>> = Mutex::new(HashMap::new());
    pub static ref receive_tick: Mutex<HashMap<usize, clock_t>> = Mutex::new(HashMap::new());
    pub static ref send_tick_instant: Mutex<HashMap<usize, Instant>> = Mutex::new(HashMap::new());
    pub static ref receive_tick_instant: Mutex<HashMap<usize, Instant>> =
        Mutex::new(HashMap::new());
}
pub mod ffi {
    extern "C" {
        pub fn clock() -> nix::libc::clock_t;
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    pub num: usize,
    pub data: Vec<u8>,
}
impl Payload {
    pub fn new(id: usize) -> Self {
        Self {
            num: id,
            data: vec![0; 5],
        }
    }
}

pub struct TimeStamp {
    pub id: usize,
    pub time_stamp: Instant,
}
pub struct TimeStampTick {
    pub id: u64,
    pub time_stamp: clock_t,
}
pub fn prepare_environment() -> PathBuf {
    // start build the rpmsg communication channel
    let virtio_id = "virtio0";
    let channel_name = "rpmsg-openamp-demo-channel";
    let version_number = "1.0";
    let device_name = format!("{}.{}.-{}", virtio_id, channel_name, version_number);
    let rpmsg_device_path = Path::new("/sys/bus/rpmsg/devices").join(device_name.clone());
    while !rpmsg_device_path.exists() {}

    // register RPMsg driver
    let rpmsg_driver_name = String::from("rpmsg_char_notify");
    register_rpmsg_driver_for_device(device_name.clone(), rpmsg_driver_name).unwrap();
    trace!("Register rpmsg driver");

    // look for control interface of character driver
    let ctrl_interface_name =
        search_control_interface(device_name.clone(), "rpmsg_ctrl".to_string()).unwrap();
    let ctrl_interface_path = Path::new("/dev").join(ctrl_interface_name.clone());
    let ctrl_interface_handler = open(&ctrl_interface_path, OFlag::O_RDWR, Mode::empty()).unwrap();

    // create endpoint
    let endpoint = RPMsgEndpointInfo::new(&channel_name, RPMSG_ADDR_ANY, 0).unwrap();
    trace!("creating endpoint: {}", channel_name);
    let ret = unsafe { ioctl_create_endpt(ctrl_interface_handler, &[endpoint]).unwrap() };
    if ret == -1 {
        panic!("can't create endpoint");
    }
    let endpoint_path_str =
        search_endpoint_path_by_name(ctrl_interface_name.clone(), channel_name.to_string())
            .unwrap();
    Path::new(&endpoint_path_str).to_path_buf()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
