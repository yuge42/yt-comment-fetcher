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

    /// Path to file containing the API key for authentication
    #[arg(long)]
    api_key_path: Option<String>,

    /// Wait time in seconds before reconnecting after connection failure (default: 5)
    #[arg(long, default_value = "5")]
    reconnect_wait_secs: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("Using video ID: {}", args.video_id);

    // Read API key from file if provided
    let api_key = if let Some(api_key_path) = &args.api_key_path {
        eprintln!("Reading API key from: {}", api_key_path);
        let key = std::fs::read_to_string(api_key_path)
            .map_err(|e| format!("Failed to read API key file '{}': {}", api_key_path, e))?
            .trim()
            .to_string();
        Some(key)
    } else {
        None
    };

    // Get REST API address from environment variable or use default
    let rest_api_address = std::env::var("REST_API_ADDRESS")
        .unwrap_or_else(|_| "https://www.googleapis.com".to_string());

    eprintln!("Fetching chat ID from REST API at: {}", rest_api_address);

    // Fetch the chat ID from the videos.list endpoint
    let chat_id = fetch_chat_id(&rest_api_address, &args.video_id, api_key.as_deref()).await?;

    eprintln!("Got chat ID: {}", chat_id);

    // Get gRPC server address from environment variable or use default
    // Note: For TLS-enabled gRPC connections, tonic requires https:// prefix
    let server_address = std::env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| "https://youtube.googleapis.com".to_string());
    let server_url =
        if server_address.starts_with("http://") || server_address.starts_with("https://") {
            server_address
        } else {
            // Default to https:// for secure connections
            format!("https://{}", server_address)
        };

    eprintln!("Connecting to gRPC server at: {}", server_url);
    eprintln!("Reconnect wait time: {} seconds", args.reconnect_wait_secs);

    // Reconnection loop
    loop {
        // Connect to the gRPC server
        let mut client = match YouTubeClient::connect(server_url.clone(), api_key.clone()).await {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Failed to connect to gRPC server: {}", e);
                eprintln!("Waiting {} seconds before reconnecting...", args.reconnect_wait_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(args.reconnect_wait_secs)).await;
                continue;
            }
        };

        // Stream comments using the retrieved chat ID
        let mut stream = match client.stream_comments(Some(chat_id.clone())).await {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to start stream: {}", e);
                eprintln!("Waiting {} seconds before reconnecting...", args.reconnect_wait_secs);
                tokio::time::sleep(tokio::time::Duration::from_secs(args.reconnect_wait_secs)).await;
                continue;
            }
        };

        // Process each message in the stream
        let mut should_reconnect = false;
        while let Some(response) = stream.next().await {
            match response {
                Ok(message) => {
                    // Print message as JSON (non-delimited)
                    let json = serde_json::to_string(&message)?;
                    println!("{}", json);
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    should_reconnect = true;
                    break;
                }
            }
        }

        // If we exited the stream loop, reconnect after waiting
        if should_reconnect {
            eprintln!("Connection lost. Waiting {} seconds before reconnecting...", args.reconnect_wait_secs);
        } else {
            eprintln!("Stream ended normally. Waiting {} seconds before reconnecting...", args.reconnect_wait_secs);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(args.reconnect_wait_secs)).await;
    }
}

async fn fetch_chat_id(
    rest_api_address: &str,
    video_id: &str,
    api_key: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut url = format!(
        "{}/youtube/v3/videos?part=liveStreamingDetails&id={}",
        rest_api_address, video_id
    );

    // Add API key as query parameter if provided
    if let Some(key) = api_key {
        url.push_str(&format!("&key={}", key));
    }

    let client = reqwest::Client::new();
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
