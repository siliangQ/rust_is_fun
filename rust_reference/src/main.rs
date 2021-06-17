/// This is a practice about reference and option in rust
/// understand the function of take
fn main() {
    let mut x = Some(10);
    let x_ref = &mut x;// mutable reference can't live with immutable reference
    let y = x_ref.take();
    println!("x: {:?}", x);
    println!("x address: {}", format!("{:p}", &x));
    println!("y: {:?}", y);
    println!("y address: {}", format!("{:p}", &y));
    x = Some(5); // it will just change the content on the memory
    println!("x new address: {}", format!("{:p}", &x));
}