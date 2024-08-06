use zest::server::zest_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    zest_main().await
}
