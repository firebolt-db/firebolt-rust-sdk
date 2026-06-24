use firebolt::FireboltClient;

fn new_firebolt_url() -> Option<String> {
    std::env::var("FIREBOLT_URL")
        .or_else(|_| std::env::var("FIREBOLT_NEW_URL"))
        .ok()
}

#[tokio::test]
async fn test_new_firebolt_discovery_flow_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let Some(url) = new_firebolt_url() else {
        println!("Skipping new Firebolt integration test: FIREBOLT_URL is not set");
        return Ok(());
    };

    let mut client = FireboltClient::builder().with_url(url).build().await?;
    let result = client.query("SELECT 42 AS answer").await?;
    let answer = result
        .rows
        .first()
        .ok_or("Expected one result row")?
        .get::<i32>("answer")?;

    assert_eq!(answer, 42);

    Ok(())
}
