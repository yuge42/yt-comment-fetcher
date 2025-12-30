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

/// Macro to handle reconnection logic (avoids code duplication)
macro_rules! handle_reconnection {
    ($reason:expr, $server_url:expr, $api_key:expr, $chat_id:expr, $page_token:expr, $reconnect_secs:expr, $stream:expr) => {{
        eprintln!("{}", $reason);

        // Log pagination status
        if let Some(ref token) = $page_token {
            eprintln!("Will resume from page token: {}", token);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs($reconnect_secs)).await;

        // Attempt to reconnect and restart stream with pagination token
        match YouTubeClient::connect($server_url.clone(), $api_key.clone()).await {
            Ok(mut new_client) => {
                match new_client
                    .stream_comments(Some($chat_id.clone()), $page_token.clone())
                    .await
                {
                    Ok(new_stream) => {
                        $stream = new_stream;
                        eprintln!("Reconnected successfully");
                    }
                    Err(e) => {
                        eprintln!("Failed to restart stream after reconnection: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to reconnect: {}", e);
            }
        }
    }};
}

/// Macro to handle stream messages (avoids code duplication)
macro_rules! handle_stream_message {
    ($stream_result:expr, $next_page_token:ident, $server_url:ident, $api_key:ident, $chat_id:ident, $reconnect_wait_secs:expr, $stream:ident) => {
        match $stream_result {
            Some(Ok(message)) => {
                // Update the page token for potential reconnection
                $next_page_token = message.next_page_token.clone();

                // Check if the response contains any items
                if message.items.is_empty() {
                    // Log empty response to stderr instead of stdout
                    eprintln!("Received empty response (no items)");
                } else {
                    // Print message as JSON (non-delimited)
                    let json = serde_json::to_string(&message)?;
                    println!("{}", json);
                }
            }
            Some(Err(e)) => {
                // Stream error (timeout or connection issue during streaming)
                let reason = format!(
                    "Error receiving message: {}\nConnection lost. Waiting {} seconds before reconnecting...",
                    e, $reconnect_wait_secs
                );
                handle_reconnection!(
                    reason,
                    $server_url,
                    $api_key,
                    $chat_id,
                    $next_page_token,
                    $reconnect_wait_secs,
                    $stream
                );
            }
            None => {
                // Stream ended (timeout or connection closed)
                let reason = format!(
                    "Stream ended. Waiting {} seconds before reconnecting...",
                    $reconnect_wait_secs
                );
                handle_reconnection!(
                    reason,
                    $server_url,
                    $api_key,
                    $chat_id,
                    $next_page_token,
                    $reconnect_wait_secs,
                    $stream
                );
            }
        }
    };
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

    // Connect to the gRPC server (fail fast if initial connection fails)
    let mut client = YouTubeClient::connect(server_url.clone(), api_key.clone()).await?;

    // Stream comments using the retrieved chat ID (fail fast if stream setup fails)
    let mut stream = client.stream_comments(Some(chat_id.clone()), None).await?;

    eprintln!("Reconnect wait time: {} seconds", args.reconnect_wait_secs);

    // Track the next page token for pagination on reconnection
    let mut next_page_token: Option<String> = None;

    // Process messages with reconnection on timeout/error
    // SIGINT (Ctrl+C) causes immediate termination
    #[cfg(unix)]
    {
        let mut sigint =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
        
        loop {
            tokio::select! {
                biased;  // Check branches in order, prioritizing signal handling
                
                _ = sigint.recv() => {
                    // Exit immediately without cleanup - this is what users expect from Ctrl+C
                    std::process::exit(130); // 128 + SIGINT (2) = 130
                }
                stream_result = stream.next() => {
                    handle_stream_message!(
                        stream_result,
                        next_page_token,
                        server_url,
                        api_key,
                        chat_id,
                        args.reconnect_wait_secs,
                        stream
                    );
                }
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        loop {
            tokio::select! {
                biased;  // Check branches in order, prioritizing signal handling
                
                _ = tokio::signal::ctrl_c() => {
                    // Exit immediately without cleanup
                    std::process::exit(130);
                }
                stream_result = stream.next() => {
                    handle_stream_message!(
                        stream_result,
                        next_page_token,
                        server_url,
                        api_key,
                        chat_id,
                        args.reconnect_wait_secs,
                        stream
                    );
                }
            }
        }
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
