extern crate actix;
use actix::{clock::Duration, prelude::*};
use actix_rt::System;
struct MyActor {}
impl Actor for MyActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(5);
    }
}
#[derive(Message)]
#[rtype(result = "()")]
struct MyMessage {}
impl Handler<MyMessage> for MyActor {
    type Result = <MyMessage as Message>::Result;

    fn handle(&mut self, msg: MyMessage, ctx: &mut Self::Context) -> Self::Result {}
}
#[actix_rt::main]
async fn main() {
    let my_actor = MyActor {}.start();
    actix::clock::delay_for(actix::clock::Duration::from_millis(100));
    let mut message_counter = 0;
    loop {
        message_counter += 1;
        if let Err(e) = my_actor.try_send(MyMessage {}) {
            panic!("{}, sent {} messages", e, message_counter);
        }
    }
    actix::clock::delay_for(actix::clock::Duration::from_millis(100)).await;
}
