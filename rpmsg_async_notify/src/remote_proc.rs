use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use nix::unistd::write;
use snafu::{ResultExt, Snafu};
use std::io;
use std::path::Path;
#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")] // Sets the default visibility for these context selectors
pub enum RemoteprocManagerError {
    #[snafu(display("fail to load {} on remoteproc, error {}", firmware_name, source))]
    FailedToLoadFirmware {
        firmware_name: String,
        source: nix::Error,
    },

    #[snafu(display("fail to {} firmware, error {}", operation, source))]
    FailedToOperateFirmware {
        operation: String,
        source: nix::Error,
    },
}
/// the manager of remote processor
/// the processor it manages is identified by remoteproc_id
/// the system that runs this manager should support remoteproc
pub struct RemoteprocManager {
    firmware_path_str: String,
    state_path_str: String,
}
impl RemoteprocManager {
    /// initialize the remoteproc manager for remoteproc_id
    pub fn new(remoteproc_id: &str) -> Result<Self, io::Error> {
        let firmware_path_str = format!("/sys/class/remoteproc/{}/firmware", remoteproc_id);
        let firmware_path = Path::new(&firmware_path_str);
        let state_path_str = format!("/sys/class/remoteproc/{}/state", remoteproc_id);
        let state_path = Path::new(&state_path_str);
        if firmware_path.exists() & state_path.exists() {
            Ok(RemoteprocManager {
                firmware_path_str,
                state_path_str,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("don't find {} on the platform", remoteproc_id),
            ))
        }
    }
    /// load specific firmware on the remoteproc
    pub fn load_firmware(&self, firmware_name: String) -> Result<(), RemoteprocManagerError> {
        let firmware_buf = firmware_name.clone().into_bytes();
        let fd = open(
            Path::new(&self.firmware_path_str),
            OFlag::O_RDWR,
            Mode::empty(),
        )
        .context(FailedToLoadFirmware {
            firmware_name: firmware_name.clone(),
        })?;
        let size = write(fd, &firmware_buf).context(FailedToLoadFirmware {
            firmware_name: firmware_name.clone(),
        })?;
        if size as usize == firmware_buf.len() {
            Ok(())
        } else {
            Err(RemoteprocManagerError::FailedToLoadFirmware {
                firmware_name,
                source: nix::Error::EINTR,
            })
        }
    }
    /// start remoteproc
    pub fn start(&self) -> Result<(), RemoteprocManagerError> {
        let command = String::from("start").into_bytes();
        let fd = open(
            Path::new(&self.state_path_str),
            OFlag::O_RDWR | OFlag::O_SYNC,
            Mode::empty(),
        )
        .context(FailedToOperateFirmware {
            operation: "start".to_string(),
        })?;
        let size = write(fd, &command).context(FailedToOperateFirmware {
            operation: "start".to_string(),
        })?;
        if size as usize == command.len() {
            Ok(())
        } else {
            Err(RemoteprocManagerError::FailedToOperateFirmware {
                operation: "start".to_string(),
                source: nix::Error::EINTR,
            })
        }
    }
    /// stop the remoteproc
    pub fn stop(&self) -> Result<(), RemoteprocManagerError> {
        let command = String::from("stop").into_bytes();
        let fd = open(
            Path::new(&self.state_path_str),
            OFlag::O_RDWR | OFlag::O_SYNC,
            Mode::empty(),
        )
        .context(FailedToOperateFirmware {
            operation: "stop".to_string(),
        })?;
        let size = write(fd, &command).context(FailedToOperateFirmware {
            operation: "stop".to_string(),
        })?;
        if size as usize == command.len() {
            Ok(())
        } else {
            Err(RemoteprocManagerError::FailedToOperateFirmware {
                operation: "stop".to_string(),
                source: nix::Error::EINTR,
            })
        }
    }
}
