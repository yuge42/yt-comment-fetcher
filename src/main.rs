use clap::Parser;
use tokio_stream::StreamExt;
use yt_grpc_client::YouTubeClient;

/// YouTube Live Comment Fetcher - Streams live chat messages from YouTube videos
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// YouTube video ID to fetch comments from
    #[arg(long, required = true)]
    video_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("Using video ID: {}", args.video_id);

    // Get REST API address from environment variable or use default
    let rest_api_address =
        std::env::var("REST_API_ADDRESS").unwrap_or_else(|_| "https://localhost:8080".to_string());

    eprintln!("Fetching chat ID from REST API at: {}", rest_api_address);

    // Fetch the chat ID from the videos.list endpoint
    let chat_id = fetch_chat_id(&rest_api_address, &args.video_id).await?;

    eprintln!("Got chat ID: {}", chat_id);

    // Get gRPC server address from environment variable or use default
    let server_address =
        std::env::var("SERVER_ADDRESS").unwrap_or_else(|_| "https://localhost:50051".to_string());
    let server_url =
        if server_address.starts_with("http://") || server_address.starts_with("https://") {
            server_address
        } else {
            format!("https://{}", server_address)
        };

    eprintln!("Connecting to gRPC server at: {}", server_url);

    // Connect to the gRPC server
    let mut client = YouTubeClient::connect(server_url).await?;

    // Stream comments using the retrieved chat ID
    let mut stream = client.stream_comments(Some(chat_id)).await?;

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

async fn fetch_chat_id(
    rest_api_address: &str,
    video_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/youtube/v3/videos?part=liveStreamingDetails&id={}",
        rest_api_address, video_id
    );

    // Build reqwest client with custom CA certificate if provided
    let client = build_http_client()?;
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await?;
        return Err(format!("Failed to fetch video data (status {}): {}", status, body).into());
    }

    let body: serde_json::Value = response.json().await?;

    // Extract the activeLiveChatId from the response
    let items = body.get("items").ok_or("Response missing 'items' field")?;

    let items_array = items.as_array().ok_or("'items' field is not an array")?;

    let first_item = items_array
        .first()
        .ok_or("No video found with the given ID")?;

    let live_streaming_details = first_item
        .get("liveStreamingDetails")
        .ok_or("Video does not have live streaming details (not a live video)")?;

    let chat_id = live_streaming_details
        .get("activeLiveChatId")
        .and_then(|id| id.as_str())
        .ok_or("No active live chat ID found (stream may not be active)")?;

    Ok(chat_id.to_string())
}

fn build_http_client() -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let mut builder = reqwest::Client::builder();
    
    // Check if a custom CA certificate path is provided
    if let Ok(ca_cert_path) = std::env::var("CA_CERT_PATH") {
        let ca_cert = std::fs::read(&ca_cert_path)?;
        let cert = reqwest::Certificate::from_pem(&ca_cert)?;
        builder = builder.add_root_certificate(cert);
        eprintln!("Using custom CA certificate from: {}", ca_cert_path);
    }
    
    Ok(builder.build()?)
}
