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

#[derive(Message)]
#[rtype(result="()")]
struct BoxMessage{
    b: Box<Vec<f32>>
}
impl Handler<BoxMessage> for MyActor{
    type Result = <BoxMessage as Message>::Result;
    fn handle(&mut self, msg: BoxMessage, ctx: &mut Self::Context) -> Self::Result{
        println!("receive box message at address: {:?}", (*msg.b).as_ptr() as *const f32);
    }
}
#[actix_rt::main]
async fn main() {
    let arbiter = Arbiter::new();
    let my_actor_obj = MyActor{};
    let my_actor = MyActor::start_in_arbiter(
        &arbiter,
        move |ctx: &mut Context<MyActor>| my_actor_obj,
    );
    actix::clock::delay_for(actix::clock::Duration::from_millis(100));
    let mut message_counter = 0;
    //loop {
        //message_counter += 1;
        ////if let Err(e) = my_actor.try_send(MyMessage {}) {
            ////panic!("{}, sent {} messages", e, message_counter);
        ////}
        //let message = BoxMessage{
            //b: Box::new(Vec::from([0.;1000 * 1000]))
        //};
        //println!("Send message at address: {:?}", (*message.b).as_ptr() as *const f32);
        //my_actor.try_send(message).await.unwrap();
    //}
    for _ in 0..5{
        let box_data =Box::new(Vec::with_capacity(1000 * 1000)); 
        println!("Send message at address: {:?}", (*box_data).as_ptr() as *const f32);
        let message = BoxMessage{
            b: box_data
        };
        my_actor.try_send(message).unwrap();
    }

    {
        let b = Box::new(5);
    }
    println!("b value: {}", b);
    actix::clock::delay_for(actix::clock::Duration::from_millis(100)).await;
}
