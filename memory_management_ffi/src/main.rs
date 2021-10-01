extern crate libc;
use libc::{c_void, c_int};
fn main() {
    let a = 130;
    let a_ptr = a as *const c_int;
    println!("pointer value: {:?}", a_ptr);
    if a_ptr.is_null(){
        println!("the pointer is null, can't dereference it");
    }else{
    unsafe{
        //if *a_ptr == 0{
            //println!("unknown memory is accessable");
        //}else{
            //println!("unknown memory is not accessable");
        //}
        println!("value: {:?}", *a_ptr);
    }
        
    }
}
