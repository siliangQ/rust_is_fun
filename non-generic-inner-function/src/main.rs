pub trait ParameterT{}
struct MyStruct;
impl ParameterT for MyStruct{}
pub fn create_inner_fun<PType: ParameterT + Sized>(p: PType){
    // This is the inner function inside a generic function
    // The compiler will not create a copy of this function when different types call the generic function
    // but if you create a function outside the generic function, different copies of inner_function will be made
    fn inner_function(){
        println!("Hello world from inner function");
    }
    println!("hello world from create_inner_fun");
    inner_function();
}
fn main() {
    println!("Hello, world!");
}
