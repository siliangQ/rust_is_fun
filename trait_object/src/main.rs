fn main() {
    // trait objects allow for multiple concrete types to fill in for the trait object at runtime
    // And there is a runtime performance implications
    println!("Hello, world!");
    // there is a runtime cost to use dynamic dispatch
    // the dynamic dispatch prevents the compiler from choosing to inline a method
}
