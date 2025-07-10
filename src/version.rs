pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROTOCOL_VERSION: &str = "2.4";

pub fn user_agent() -> String {
    format!("Rust SDK {VERSION}")
}
