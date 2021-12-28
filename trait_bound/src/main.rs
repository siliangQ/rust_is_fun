use std::collections::HashMap;
use std::iter::FromIterator;
fn generate_hashmap<K, S>() -> HashMap<K, usize, S>
where
    HashMap<K, usize, S>: FromIterator<(String, usize)>
{
    let map: HashMap<String, usize> = HashMap::new();
    map

}
fn main() {
    println!("Hello, world!");
}
