mod common;

use common::{validate_environment, TestConfig};

#[allow(dead_code)]
fn setup() -> Result<TestConfig, String> {
    validate_environment()?;
    TestConfig::from_env()
}
