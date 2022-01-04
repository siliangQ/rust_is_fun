use log::{error, trace, warn};
use nix::fcntl::{open, OFlag};
use nix::ioctl_write_buf;
use nix::libc::{__u32, c_char};
use nix::sys::stat::Mode;
use nix::unistd::{access, close, read, write, AccessFlags};
use snafu::ResultExt;
use snafu::Snafu;
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::{self, prelude::*};
use std::os::unix::prelude::RawFd;
use std::path::Path;

const RPMSG_ADDR_ANY: __u32 = 0xffffffff;
#[derive(Snafu, Debug, Clone, PartialEq)]
#[snafu(visibility = "pub")]
pub enum SystemFileHandlerError {
    #[snafu(display("failed to open {}, error {:?}", fpath, source))]
    FailedToOpen { fpath: String, source: nix::Error },
    #[snafu(display("failed to write {} to {}, error {:?}", data, fpath, source))]
    FailedToWrite {
        data: String,
        fpath: String,
        source: nix::Error,
    },
    #[snafu(display("failed to close {}, error {}", fpath, source))]
    FailedToClose { fpath: String, source: nix::Error },
}

#[derive(Debug, Snafu, Clone, PartialEq)]
#[snafu(visibility = "pub")]
pub enum ChannelError {
    /// can't access some device in the system
    #[snafu(display("not able to access {}", device_name))]
    FailToAccessDevice { device_name: String },
    /// can't open directory in the system, a wrapper for IO Error
    #[snafu(display("failed to open directory {}", dir_path))]
    FailedToOpenDir { dir_path: String },
    /// can't read directory in the system, a wrapper for IO Error
    #[snafu(display("failed to read directory {}", dir_path))]
    FailedToReadDir { dir_path: String },
    /// can't find control interface exposed by driver
    #[snafu(display("no control interface found in {}", dir_path))]
    FailedToFindCtrlInterface { dir_path: String },
    /// can't open the file specified by path
    #[snafu(display("can't open {}, error: {}", path, source))]
    FailedToOpenFileError { path: String, source: nix::Error },
    /// can't write to the file specifed by path
    #[snafu(display("can't write to {}, error: {}", path, source))]
    FailedToWriteFileError { path: String, source: nix::Error },
    #[snafu(display("{:?}", source))]
    #[snafu(context(false))]
    SysError { source: nix::Error },
    /// convert str::Utf8Error into a channel error
    #[snafu(display("endpoint name is invalid, error {:?}", source))]
    #[snafu(context(false))]
    InvalidEndpointName { source: std::str::Utf8Error },
    /// can't create a valid endpoint in the search path
    #[snafu(display("failed to find endpoint {}", endpoint_name))]
    FailedToCreateEndpoint { endpoint_name: String },
    /// can't convert OsStr to String
    #[snafu(display("failed to convert to string in rust"))]
    OsStrConversion {},
    /// a wrapper for I/O error in std because the io::Error is not clonable
    #[snafu(display("{:?}", error))]
    IOError { error: String },
    /// a wrapper for SystemFileHanderError
    #[snafu(display("{:?}", source))]
    #[snafu(context(false))]
    SFHError { source: SystemFileHandlerError },
    /// can't send the data through
    #[snafu(display("can't send data through enpoint {}", endpoint_name))]
    FailedToSend { endpoint_name: String },
    /// failed to convert from u8 to i8
    #[snafu(display("can't convert a u8 to i8"))]
    FailedToConvertU8ToI8 { num: u8 },
    /// message overflow the buffer
    MessageBufferOverflow {
        capacity: usize,
        message_size: usize,
    },
}

/// struct of rpmsg endpoint information
/// We send this data structure to kernel, it requires a C-like struct
#[derive(Clone, Debug)]
#[repr(C)]
pub struct RPMsgEndpointInfo {
    // c_char in aarch64 is u8 in rust
    // c_char in x86 is i8 in rust
    name: [c_char; 32],
    src: __u32,
    dst: __u32,
}
impl RPMsgEndpointInfo {
    pub fn new(channel_name: &str, src: u32, dst: u32) -> Result<RPMsgEndpointInfo, ChannelError> {
        let mut eptinfo = RPMsgEndpointInfo {
            name: [0; 32],
            src,
            dst,
        };
        trace!("Endpoint created in unknown architecture, try to be compatiable");
        // method 1 to convert a u8 to c_char(copy)
        let _channel_name_i8: Vec<c_char> = channel_name
            .as_bytes()
            .iter()
            .map(|c| {
                if *c > 127 {
                    Err(ChannelError::FailedToConvertU8ToI8 { num: *c })
                } else {
                    Ok(*c as c_char)
                }
            })
            .collect::<Result<Vec<c_char>, ChannelError>>()?;

        // method 2: unsafe to convert u8 to c_char(no copy)
        // an unsafe way to convert &[u8] to &[i8] since the FFI C function takes c_char(aliased to u8 or i8 in rust)
        // this will have problem when we have character in channel_name which is encoded larger than 127.
        // this data is sent to linux kernel, I don't know if kernel will be able to handle it properly
        let bytes = unsafe { &*(channel_name.as_bytes() as *const [u8] as *const [c_char]) };
        eptinfo.name[0..bytes.len()].copy_from_slice(bytes);
        Ok(eptinfo)
    }
}
// create a function to use ioctl system call for creating a endpoint
ioctl_write_buf!(ioctl_create_endpt, 0xb5, 0x1, RPMsgEndpointInfo);

/// the struct of endpoint created by rpmsg character driver
#[derive(Debug, Clone)]
pub struct RPMsgEndpoint {
    // the path to the interface in system
    name: String,
    // initialized handler of the interface
    // the default value is none
    endpoint_handler: i32,
}

impl RPMsgEndpoint {
    // initiate the endpoint with path to interface in the system
    pub fn new(endpoint_name: String) -> Result<RPMsgEndpoint, ChannelError> {
        let endpoint_path = Path::new(&endpoint_name);
        trace!("creating endpoint :{:?}", endpoint_path);
        if endpoint_path.exists() {
            trace!("opening endpoint handler");
            let endpoint_handler = open(
                endpoint_path,
                OFlag::O_RDWR | OFlag::O_NONBLOCK,
                Mode::empty(),
            )
            .context(FailedToOpenFileError {
                path: endpoint_name.clone(),
            })?;
            Ok(RPMsgEndpoint {
                name: endpoint_name,
                endpoint_handler,
            })
        } else {
            Err(ChannelError::FailedToCreateEndpoint { endpoint_name })
        }
    }
    // send the message through endpoint, return Channel Error
    pub fn send(&mut self, message: &[u8]) -> Result<(), ChannelError> {
        trace!("sending through the enpoint: {} bytes", message.len());
        let bytes_sent = write(self.endpoint_handler, message)?;
        if bytes_sent != message.len() {
            return Err(ChannelError::FailedToSend {
                endpoint_name: self.name.clone(),
            });
        }
        Ok(())
    }
    /// try to read the message from endpoint and return the message buffer
    /// the capacity set the largest size of message we could receive
    /// return error when there is no data or endpoint is not working
    pub fn read(&mut self, capacity: usize) -> Result<Vec<u8>, ChannelError> {
        //let mut buf: Vec<u8> = Vec::with_capacity(capacity);
        let mut buf = vec![0; 1024];
        let size = read(self.endpoint_handler, &mut buf)?;
        if size > capacity {
            Err(ChannelError::MessageBufferOverflow {
                capacity,
                message_size: size,
            })
        } else {
            // why: vector was created with length 0, now set it to the length of message
            unsafe {
                buf.set_len(size);
            }
            Ok(buf)
        }
    }
}

/// # Abstract
/// This trait defines the contract for Channel in a generic fashion
pub trait AbstractRPMsgChannel: Sized {
    fn instantiate(
        device_name: String,
        virtio_id: String,
        version_number: String,
    ) -> Result<Self, ChannelError>;
    fn send(&mut self, message: &[u8]) -> Result<(), ChannelError>;
    fn read(&mut self, capacity: usize) -> Result<Vec<u8>, ChannelError>;
}

pub struct OctRPMsgChannel {
    // the name of connected rpmsg_device
    _rpmsg_device_name: String,
    // the name of control interface
    _ctrl_interface_name: String,
    // the handler to control interface
    _ctrl_interface_handler: i32,
    // the endpoint for message passing
    endpoint: RPMsgEndpoint,
}

/// register the driver specified by driver_name to the device specified by device_name
pub fn register_rpmsg_driver_for_device(
    device_name: String,
    driver_name: String,
) -> Result<(), io::Error> {
    let driver_api = format!("/sys/bus/rpmsg/devices/{}/driver_override", device_name);
    let mut fd = OpenOptions::new().write(true).open(driver_api).unwrap();
    fd.write_all(driver_name.as_bytes())?;
    let driver_bind_api = format!("/sys/bus/rpmsg/drivers/{}/bind", driver_name);
    let mut fd = OpenOptions::new()
        .write(true)
        .open(driver_bind_api)
        .unwrap();
    fd.write_all(device_name.as_bytes())?;
    Ok(())
}

/// search the control interface exposed by the driver to the device specified by device_name
pub fn search_control_interface(
    device_name: String,
    ctrl_prefix: String,
) -> Result<String, io::Error> {
    let ctrl_interface_dir_path_str = format!("/sys/bus/rpmsg/devices/{}/rpmsg", device_name);
    let ctrl_interface_dir_path = Path::new(&ctrl_interface_dir_path_str);
    let dir_content = fs::read_dir(ctrl_interface_dir_path)?;
    for entry in dir_content {
        let path = entry?.path();

        if let Some(path_str) = path.to_str() {
            if path_str.contains(&ctrl_prefix) {
                if let Some(interface_id) = path.file_name() {
                    return Ok(interface_id
                        .to_str()
                        .ok_or(io::ErrorKind::InvalidData {})?
                        .to_string());
                } else {
                    error!("can't get interface id from path {:?}", path);
                    continue;
                }
            }
        } else {
            error!("can't convert {:?} to string", path);
            continue;
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "can't find the right control interface",
    ))
}

/// seach the endpoint name created by control interface.(I don't know why we need to search instead of get it from somewhere)
pub fn search_endpoint_path_by_name(
    ctrl_interface_name: String,
    endpoint_name: String,
) -> Result<String, ChannelError> {
    for i in 0..128 {
        let rpmsg_ept_name_registry_path_str =
            format!("/sys/class/rpmsg/{0}/rpmsg{1}/name", ctrl_interface_name, i);
        let rpmsg_ept_name_registry_path = Path::new(&rpmsg_ept_name_registry_path_str);
        if access(rpmsg_ept_name_registry_path, AccessFlags::F_OK).is_err() {
            continue;
        }

        // fetch name of candidate endpoint
        let mut fd = OpenOptions::new()
            .read(true)
            .open(rpmsg_ept_name_registry_path)
            .unwrap();
        let mut candidate_endpoint = String::new();
        fd.read_to_string(&mut candidate_endpoint)
            .map_err(|e| ChannelError::IOError {
                error: format!("{:?}", e),
            })?;
        // the name extracted from the interface appended with \n
        if candidate_endpoint.ends_with('\n') {
            candidate_endpoint.pop();
        }
        //println!("candidate endpoint: {:?}", candidate_endpoint);
        //println!("target endpoint: {:?}", endpoint_name);
        if endpoint_name.eq(&candidate_endpoint) {
            trace!("found path for enpoint {}", endpoint_name);
            return Ok(format!("/dev/rpmsg{}", i));
        }
    }
    Err(ChannelError::FailedToCreateEndpoint { endpoint_name })
}

impl AbstractRPMsgChannel for OctRPMsgChannel {
    fn instantiate(
        channel_name: String,
        virtio_id: String,
        version_number: String,
    ) -> Result<Self, ChannelError> {
        trace!("Open rpmsg dev {}", channel_name);
        let device_name = format!("{}.{}.-{}", virtio_id, channel_name, version_number);
        let rpmsg_device_path = Path::new("/sys/bus/rpmsg/devices").join(device_name.clone());

        // condition compilation for waitting RPU response
        if cfg!(test) || cfg!(feature = "debug") {
            while access(&rpmsg_device_path, AccessFlags::F_OK).is_err() {}
        } else if access(&rpmsg_device_path, AccessFlags::F_OK).is_err() {
            return Err(ChannelError::FailToAccessDevice { device_name });
        }

        // register RPMsg driver
        let rpmsg_driver_name = String::from("rpmsg_chrdev");
        register_rpmsg_driver_for_device(device_name.clone(), rpmsg_driver_name).unwrap();
        trace!("Register rpmsg driver");

        // look for control interface of character driver
        let ctrl_interface_name =
            search_control_interface(device_name.clone(), "rpmsg_ctrl".to_string()).unwrap();
        let ctrl_interface_path = Path::new("/dev").join(ctrl_interface_name.clone());
        let ctrl_interface_handler = open(&ctrl_interface_path, OFlag::O_RDWR, Mode::empty())?;

        // create endpoint
        let endpoint = RPMsgEndpointInfo::new(&channel_name, RPMSG_ADDR_ANY, RPMSG_ADDR_ANY)?;
        trace!("creating endpoint: {}", channel_name);
        let ret = unsafe { ioctl_create_endpt(ctrl_interface_handler, &[endpoint])? };
        if ret == -1 {
            return Err(ChannelError::FailedToCreateEndpoint {
                endpoint_name: channel_name,
            });
        }
        let endpoint_path_str =
            search_endpoint_path_by_name(ctrl_interface_name.clone(), channel_name)?;
        let endpoint = RPMsgEndpoint::new(endpoint_path_str)?;

        Ok(OctRPMsgChannel {
            _rpmsg_device_name: device_name,
            _ctrl_interface_name: ctrl_interface_name,
            _ctrl_interface_handler: ctrl_interface_handler,
            endpoint,
        })
    }

    fn send(&mut self, message: &[u8]) -> Result<(), ChannelError> {
        self.endpoint.send(message)
    }

    /// a wrapper function around endpoint read api
    fn read(&mut self, capacity: usize) -> Result<Vec<u8>, ChannelError> {
        self.endpoint.read(capacity)
    }
}
