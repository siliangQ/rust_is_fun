extern crate actix;
use actix::prelude::*;
struct MyActor;
impl Actor for MyActor{
    type Context=Context<Self>;
}

#[actix_rt::main]
async fn main() {
    let act_addr = MyActor.start();
    {
        // the drop of address will not stop the actor
       let act_addr_clone = act_addr.clone();
    }
    println!("Hello, world!");
}
