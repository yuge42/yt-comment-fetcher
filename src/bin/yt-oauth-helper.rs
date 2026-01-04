use clap::Parser;
use yt_oauth::{OAuthConfig, start_auth_flow};

/// OAuth 2.0 helper tool for YouTube API authentication
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// OAuth client ID
    #[arg(long, required = true)]
    client_id: String,

    /// OAuth client secret
    #[arg(long, required = true)]
    client_secret: String,

    /// Path to save the OAuth token file
    #[arg(long, required = true)]
    token_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config = OAuthConfig::new(args.client_id, args.client_secret);

    // Start OAuth authorization flow
    let token = start_auth_flow(&config).await?;

    // Save token to file
    token.save_to_file(&args.token_path)?;

    eprintln!("\nOAuth token saved to: {}", args.token_path);
    eprintln!("You can now use this token with yt-comment-fetcher");

    Ok(())
}
