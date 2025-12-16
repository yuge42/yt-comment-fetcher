use tokio_stream::StreamExt;
use yt_grpc_client::YouTubeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the mock server
    let mut client = YouTubeClient::connect("http://localhost:50051".to_string()).await?;

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
    println!("Connection ended");

    Ok(())
}
