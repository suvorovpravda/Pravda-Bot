#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pravda_server::run_server().await
}
