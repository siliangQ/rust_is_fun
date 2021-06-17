#[macro_use]
extern crate lazy_static;
mod settings;
lazy_static! {
    static ref CONFIG: settings::Settings = settings::Settings::new().expect("can't load config");
}
fn main() {
    println!("config url: {}", CONFIG.server.port);
}
