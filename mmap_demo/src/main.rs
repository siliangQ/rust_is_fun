use nix::fcntl::open;
use nix::fcntl::OFlag;
use nix::libc::{c_char, c_uchar};
use nix::sys::mman::{mmap, MapFlags, ProtFlags};
use nix::sys::stat::Mode;
use std::ffi::c_void;
use std::ptr;

fn main() {
    let pl_ddr_phy_base_addr = 0x400000000 as i64;
    let memfd = open("/dev/mem", OFlag::O_RDWR | OFlag::O_SYNC, Mode::empty()).unwrap();
    let buffer_size = 0x1000;
    unsafe {
        let pl_ddr_virt_base_addr = mmap(
            ptr::null_mut() as *mut c_void,
            buffer_size,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED,
            memfd,
            pl_ddr_phy_base_addr,
        )
        .unwrap();
        // read and write to the memory
        println!(
            "read from memory(before): {:#x}",
            *(pl_ddr_virt_base_addr as *mut c_uchar)
        );
        *(pl_ddr_virt_base_addr as *mut c_uchar) = 0xDD;
        println!(
            "read from memory(after): {:#x}",
            *(pl_ddr_virt_base_addr as *mut c_uchar)
        );
    }
}
