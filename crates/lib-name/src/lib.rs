pub const HELLO_WORLD_STRING: &str = "Hello World!";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_world_test() {
        let hello_world = "Hello World!";
        let goodbye_world = "Goodbye World!";
        assert_eq!(HELLO_WORLD_STRING, hello_world);
        assert_ne!(HELLO_WORLD_STRING, goodbye_world);
    }
}
