#[cfg(test)]
mod tests {
    use std::{
        ffi::{CStr, CString},
        mem,
        os::raw::c_char,
    };
    struct Alignment {
        id: u8,
        count: f64,
    }
    #[test]
    fn check_memory_alignment() {
        // because of memory alignment the size of Alignment is 16 bytes instead of 9 bytes
        let mem_size = mem::size_of::<Alignment>();
        println!("memory size: {}", mem_size);
        assert_eq!(mem_size, 16);
    }
    #[test]
    fn raw_pointer_ownership() {
        let mut s1 = "hello world".to_string();
        // double free detected
        //let s1_ptr = s1.as_mut_ptr() as *mut c_char;
        //unsafe {
        //let s1_cstring = CString::from_raw(s1_ptr);
        //}
        //unsafe {
        //println!("pointer: {}", *s1_ptr);
        //}
        // reference is safer?
        let s1_const_str = s1.as_ptr() as *const c_char;
        unsafe {
            let s1_str = CStr::from_ptr(s1_const_str);
            println!("the string is: {:?}", s1_str);
        }
        unsafe {
            println!("the pointer is : {}", *s1_const_str);
        }
    }
}
