#[cfg(test)]
mod tests {
    use std::process::Command;
    #[test]
    fn it_works() {
        let mut command = Command::new("echo");
        command.arg("hello world");
        let result = command.output();
        println!("{:?}", result);
    }
}
