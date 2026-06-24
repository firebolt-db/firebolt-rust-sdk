pub mod client_credentials;

pub use client_credentials::authenticate;
pub(crate) use client_credentials::authenticate_with_client;
