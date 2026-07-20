trait Hello {
    fn say_hello(name: &str);
}
struct MyStruct;
impl Hello for MyStruct {
    fn say_hello(name: &str) {
        println!("Hello {}!", name);
    }
}
