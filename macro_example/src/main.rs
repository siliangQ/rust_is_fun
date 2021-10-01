macro_rules! impl_from_for_command_and_response {
    ($message_name:ident {
        $($subtype:ty),+
    }) => {
        paste!{
            $(impl From<$subtype> for $message_name{
                fn from(cmd: $subtype) -> Self{
                    Self{
                        $message_name
                    }
                }
            })+
        }
    };
}
fn main() {
    println!("Hello, world!");
}
