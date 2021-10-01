use rand;
use rand::Rng;
/// This is a practice about reference and option in rust
/// understand the function of take
fn move_value_behind_mut_reference(s: &mut Box<i32>) {
    let was = std::mem::take(s); // was (Box<i32>), takes the value behind a refernce and leave a default value
                                 // the default value of i32 is 0
                                 //let wrong_was = *s;
}

// lifetime of reference
fn reference_lifetime_2_8() {
    let mut x = Box::new(42);
    let r = &x;
    let mut rng = rand::thread_rng();
    if rng.gen::<f32>() > 0.5 {
        *x = 84;
    } else {
        println!("{}", r);
    }
    //println!("outside r: {}", r);
}

// reassign the lifetime when it is not valid
fn reassign_reference() {
    let mut x = Box::new(42);
    let mut r = &x;
    for _ in 0..1 {
        println!("r: {}", r);
        x = Box::new(10);
        r = &x;
        println!("r: {}", r);
    }
    println!("r(end): {}", r);
}
fn main() {
    reassign_reference();
    /*
    reference_lifetime_2_8();
    let mut s = Box::new(10);
    move_value_behind_mut_reference(&mut s);
    println!("{}", s);
    let mut x = Some(10);
    let x_ref = &mut x; // mutable reference can't live with immutable reference
    let y = x_ref.take();
    println!("x: {:?}", x);
    println!("x address: {}", format!("{:p}", &x));
    println!("y: {:?}", y);
    println!("y address: {}", format!("{:p}", &y));
    x = Some(5); // it will just change the content on the memory
    println!("x new address: {}", format!("{:p}", &x));
    */
}
