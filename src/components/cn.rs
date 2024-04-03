pub mod channel {
    pub const NAME: &str = "CN";

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub struct Channel {
        name: String,
        source: String,
    }
}