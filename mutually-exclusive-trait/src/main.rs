/// This is a idea from this blog(https://geo-ant.github.io/blog/2021/mutually-exclusive-traits-rust/). 
/// all credits given to that blog
/// Please refer to it if you have any problem

use std::fmt::Debug;
trait LogLevel{}
struct Error;
struct Info;
impl LogLevel for Error {}
impl LogLevel for Info {}
trait Task{}
trait LogTask: Task + Debug{
    type Level: LogLevel;
}
// implement two types of logging tasks
#[derive(Debug)]
struct ErrorMessage(String);
impl Task for ErrorMessage{}
impl LogTask for ErrorMessage{
    type Level = Error;
}

#[derive(Debug)]
struct InfoMessage(String);
impl Task for InfoMessage{}
impl LogTask for InfoMessage{
    type Level = Info;
}

trait Executor<T:Task>{
    fn handle(&mut self, task: T);
}
struct Logger;

/* Hopefully It could work with the compiler, but [this compiler error](https://github.com/rust-lang/rust/issues/20400)
We need to have a workaround, remove the comment if you want to see the error*/
//impl<I> Executor<I> for Logger
//where I: LogTask<Level=Info>
//{
    //fn handle(&mut self, task: I){
        //println!("Info: {}", task);
    //}
//}
//impl<E> Executor<E> for Logger
//where E: LogTask<Level=Error>
//{
    //fn handle(&mut self, task: E){
        //println!("Error: {}", task);
    //}
//}

/* Workaround solution */
trait LogExecutor<T: LogTask, L=<T as LogTask>::Level> {
    fn log_handle(&mut self, task: T);
}

impl<T> LogExecutor<T, Error> for Logger
where
    T: LogTask<Level=Error>
{
    fn log_handle(&mut self, task: T) {
        println!("Error: {:?}", task);
    }

}

impl<T> LogExecutor<T, Info> for Logger
where
    T: LogTask<Level=Info>
    {
        fn log_handle(&mut self, task: T) {
            println!("Info: {:?}", task);
        }
    }

impl<T> Executor<T> for Logger
where
    T: LogTask,
    Self: LogExecutor<T> {
        fn handle(&mut self, task: T){
            self.log_handle(task);
        }
    }

fn main() {
    let mut logger = Logger{};
    logger.handle(ErrorMessage("this is a error".to_string()));
    logger.handle(InfoMessage("this is a info".to_string()));
}