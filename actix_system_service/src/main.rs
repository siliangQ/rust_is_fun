use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
struct Ping;

#[derive(Default, Debug)]
struct MyActor1;

impl Actor for MyActor1 {
    type Context = Context<Self>;
}
impl actix::Supervised for MyActor1 {
    fn restarting(&mut self, ctx: &mut Self::Context) {
        println!("restarting actor");
    }
}

impl SystemService for MyActor1 {
    fn service_started(&mut self, ctx: &mut Context<Self>) {
        println!("Service started");
    }
}

impl Handler<Ping> for MyActor1 {
    type Result = ();

    fn handle(&mut self, _: Ping, ctx: &mut Context<Self>) {
        println!("ping");
        ctx.stop();
        System::current().stop();
    }
}

#[derive(Debug)]
struct MyActor2;

impl Actor for MyActor2 {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Context<Self>) {}
}
impl Handler<Ping> for MyActor2 {
    type Result = <Ping as Message>::Result;

    fn handle(&mut self, msg: Ping, ctx: &mut Self::Context) -> Self::Result {
        let act = MyActor1::from_registry();
        println!("The address of actor1: {:?}", act);
        act.do_send(Ping);
    }
}
fn main() {
    let system = System::new("system-service-example");

    // Define an execution flow using futures
    let execution = async {
        let act = MyActor2 {}.start();
        act.send(Ping).await.unwrap();
    };

    // Spawn the future onto the current Arbiter/event loop
    Arbiter::spawn(execution);
    system.run().unwrap();
}
