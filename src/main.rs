use tokio_stream::StreamExt;
use yt_grpc_client::YouTubeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get server address from environment variable or use default
    let server_address =
        std::env::var("SERVER_ADDRESS").unwrap_or_else(|_| "localhost:50051".to_string());
    let server_url =
        if server_address.starts_with("http://") || server_address.starts_with("https://") {
            server_address
        } else {
            format!("http://{}", server_address)
        };

    // Connect to the server
    let mut client = YouTubeClient::connect(server_url).await?;

    // Stream comments
    let mut stream = client.stream_comments(None).await?;

    // Process each message in the stream
    while let Some(response) = stream.next().await {
        match response {
            Ok(message) => {
                // Print message as JSON (non-delimited)
                let json = serde_json::to_string(&message)?;
                println!("{}", json);
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
        }
    }

    // Print message when connection ends
    eprintln!("Connection ended");

    Ok(())
}
