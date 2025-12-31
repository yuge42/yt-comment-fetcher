use clap::Parser;
use std::fs::OpenOptions;
use std::io::Write;
use tokio_stream::StreamExt;
use yt_grpc_client::YouTubeClient;

/// YouTube Live Comment Fetcher - Streams live chat messages from YouTube videos
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// YouTube video ID to fetch comments from (optional when --resume is used)
    #[arg(long)]
    video_id: Option<String>,

    /// Path to file containing the API key for authentication
    #[arg(long)]
    api_key_path: Option<String>,

    /// Wait time in seconds before reconnecting after connection failure (default: 5)
    #[arg(long, default_value = "5")]
    reconnect_wait_secs: u64,

    /// Path to output file where JSON messages will be written (one per line)
    #[arg(long)]
    output_file: Option<String>,

    /// Resume streaming from the last message in the output file
    #[arg(long)]
    resume: bool,
}

/// Macro to attempt reconnection and restart stream
macro_rules! attempt_reconnect {
    ($server_url:expr, $api_key:expr, $chat_id:expr, $page_token:expr, $stream:expr, $reconnect_until:expr, $reconnect_secs:expr) => {{
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
                        // Schedule another reconnection attempt
                        $reconnect_until = Some(
                            tokio::time::Instant::now()
                                + tokio::time::Duration::from_secs($reconnect_secs),
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to reconnect: {}", e);
                // Schedule another reconnection attempt
                $reconnect_until = Some(
                    tokio::time::Instant::now() + tokio::time::Duration::from_secs($reconnect_secs),
                );
            }
        }
    }};
}

/// Macro to handle stream messages (avoids code duplication)
macro_rules! handle_stream_message {
    ($stream_result:expr, $next_page_token:ident, $reconnect_until:ident, $reconnect_wait_secs:expr, $output_file:expr) => {
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

                    // Write to file or stdout
                    if let Some(ref mut file) = $output_file {
                        writeln!(file, "{}", json)?;
                        file.flush()?;
                    } else {
                        println!("{}", json);
                    }
                }
            }
            Some(Err(e)) => {
                // Stream error (timeout or connection issue during streaming)
                eprintln!(
                    "Error receiving message: {}\nConnection lost. Waiting {} seconds before reconnecting...",
                    e, $reconnect_wait_secs
                );

                // Log pagination status
                if let Some(ref token) = $next_page_token {
                    eprintln!("Will resume from page token: {}", token);
                }

                // Schedule reconnection
                $reconnect_until = Some(
                    tokio::time::Instant::now()
                        + tokio::time::Duration::from_secs($reconnect_wait_secs),
                );
            }
            None => {
                // Stream ended (timeout or connection closed)
                eprintln!(
                    "Stream ended. Waiting {} seconds before reconnecting...",
                    $reconnect_wait_secs
                );

                // Log pagination status
                if let Some(ref token) = $next_page_token {
                    eprintln!("Will resume from page token: {}", token);
                }

                // Schedule reconnection
                $reconnect_until = Some(
                    tokio::time::Instant::now()
                        + tokio::time::Duration::from_secs($reconnect_wait_secs),
                );
            }
        }
    };
}

/// Read the last line from a file
fn read_last_line(path: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    use std::io::{BufRead, BufReader};

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let reader = BufReader::new(file);
    let mut last_line: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            last_line = Some(line);
        }
    }

    Ok(last_line)
}

/// Parse resume information from the last JSON line
fn parse_resume_info(
    json_line: &str,
) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_str(json_line)?;

    // Extract live_chat_id from items[0].snippet.live_chat_id
    let chat_id = value
        .get("items")
        .and_then(|items| items.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("snippet"))
        .and_then(|snippet| snippet.get("liveChatId"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string());

    // Extract next_page_token
    let next_page_token = value
        .get("nextPageToken")
        .and_then(|token| token.as_str())
        .map(|s| s.to_string());

    Ok((chat_id, next_page_token))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Validate arguments
    if !args.resume && args.video_id.is_none() {
        return Err("Either --video-id or --resume must be specified".into());
    }

    if args.resume && args.output_file.is_none() {
        return Err("--output-file must be specified when using --resume".into());
    }

    // Read API key from file if provided (needed for both REST and gRPC)
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

    // Open output file if specified
    let mut output_file = if let Some(ref path) = args.output_file {
        eprintln!("Output file: {}", path);
        Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("Failed to open output file '{}': {}", path, e))?,
        )
    } else {
        None
    };

    // Try to resume from file if requested
    let (mut chat_id, initial_page_token) = if args.resume {
        let output_path = args.output_file.as_ref().unwrap();
        eprintln!("Attempting to resume from: {}", output_path);

        match read_last_line(output_path)? {
            Some(last_line) => {
                eprintln!("Found last line, parsing resume info...");
                match parse_resume_info(&last_line) {
                    Ok((Some(cid), token)) => {
                        eprintln!("Resuming with chat ID: {}", cid);
                        if let Some(ref t) = token {
                            eprintln!("Resuming from page token: {}", t);
                        }
                        (Some(cid), token)
                    }
                    Ok((None, _)) => {
                        eprintln!("Could not extract chat ID from last line");
                        (None, None)
                    }
                    Err(e) => {
                        eprintln!("Failed to parse last line: {}", e);
                        (None, None)
                    }
                }
            }
            None => {
                eprintln!("Output file is empty or does not exist yet");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    // If we don't have a chat_id from resume, fetch it using video_id
    if chat_id.is_none() {
        let video_id = args
            .video_id
            .as_ref()
            .ok_or("video-id is required when not resuming or when resume fails to find chat ID")?;
        eprintln!("Using video ID: {}", video_id);

        // Get REST API address from environment variable or use default
        let rest_api_address = std::env::var("REST_API_ADDRESS")
            .unwrap_or_else(|_| "https://www.googleapis.com".to_string());

        eprintln!("Fetching chat ID from REST API at: {}", rest_api_address);

        // Fetch the chat ID from the videos.list endpoint
        chat_id = Some(fetch_chat_id(&rest_api_address, video_id, api_key.as_deref()).await?);

        eprintln!("Got chat ID: {}", chat_id.as_ref().unwrap());
    }

    let chat_id = chat_id.unwrap(); // Safe because we've ensured chat_id is Some at this point

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

    // Stream comments using the retrieved chat ID and page token (if resuming)
    let mut stream = client
        .stream_comments(Some(chat_id.clone()), initial_page_token.clone())
        .await?;

    eprintln!("Reconnect wait time: {} seconds", args.reconnect_wait_secs);

    // Track the next page token for pagination on reconnection
    // Initialize with the resume token if we have one
    let mut next_page_token: Option<String> = initial_page_token;

    // Track when we should attempt reconnection (None means we're connected)
    let mut reconnect_until: Option<tokio::time::Instant> = None;

    // Process messages with reconnection on timeout/error and signal handling
    #[cfg(unix)]
    {
        // Unix: Handle both SIGINT and SIGTERM
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

        loop {
            // If we're scheduled to reconnect, wait until the time arrives
            if let Some(until) = reconnect_until {
                tokio::select! {
                    // Wait for the reconnection delay
                    _ = tokio::time::sleep_until(until) => {
                        // Time to reconnect
                        reconnect_until = None;

                        attempt_reconnect!(
                            server_url,
                            api_key,
                            chat_id,
                            next_page_token,
                            stream,
                            reconnect_until,
                            args.reconnect_wait_secs
                        );
                    }
                    // Handle SIGINT (Ctrl+C) - immediate exit even during reconnect wait
                    _ = tokio::signal::ctrl_c() => {
                        eprintln!("Received SIGINT, shutting down...");
                        break;
                    }
                    // Handle SIGTERM - immediate exit even during reconnect wait
                    _ = sigterm.recv() => {
                        eprintln!("Received SIGTERM, shutting down...");
                        break;
                    }
                }
            } else {
                // Normal operation - process stream messages
                tokio::select! {
                    // Handle incoming messages from the stream
                    stream_result = stream.next() => {
                        handle_stream_message!(
                            stream_result,
                            next_page_token,
                            reconnect_until,
                            args.reconnect_wait_secs,
                            output_file
                        );
                    }
                    // Handle SIGINT (Ctrl+C)
                    _ = tokio::signal::ctrl_c() => {
                        eprintln!("Received SIGINT, shutting down...");
                        break;
                    }
                    // Handle SIGTERM
                    _ = sigterm.recv() => {
                        eprintln!("Received SIGTERM, shutting down...");
                        break;
                    }
                }
            }
        }
    }

    #[cfg(not(unix))]
    {
        // Non-Unix (e.g., Windows): Handle only SIGINT/Ctrl+C
        loop {
            // If we're scheduled to reconnect, wait until the time arrives
            if let Some(until) = reconnect_until {
                tokio::select! {
                    // Wait for the reconnection delay
                    _ = tokio::time::sleep_until(until) => {
                        // Time to reconnect
                        reconnect_until = None;

                        attempt_reconnect!(
                            server_url,
                            api_key,
                            chat_id,
                            next_page_token,
                            stream,
                            reconnect_until,
                            args.reconnect_wait_secs
                        );
                    }
                    // Handle SIGINT (Ctrl+C) - immediate exit even during reconnect wait
                    _ = tokio::signal::ctrl_c() => {
                        eprintln!("Received SIGINT, shutting down...");
                        break;
                    }
                }
            } else {
                // Normal operation - process stream messages
                tokio::select! {
                    // Handle incoming messages from the stream
                    stream_result = stream.next() => {
                        handle_stream_message!(
                            stream_result,
                            next_page_token,
                            reconnect_until,
                            args.reconnect_wait_secs,
                            output_file
                        );
                    }
                    // Handle SIGINT (Ctrl+C)
                    _ = tokio::signal::ctrl_c() => {
                        eprintln!("Received SIGINT, shutting down...");
                        break;
                    }
                }
            }
        }
    }

    eprintln!("Shutdown complete");
    Ok(())
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
