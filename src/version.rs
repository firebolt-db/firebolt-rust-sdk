pub const VERSION: &str = "0.0.1";

pub fn user_agent() -> String {
    format!("Rust SDK {}", VERSION)
}
