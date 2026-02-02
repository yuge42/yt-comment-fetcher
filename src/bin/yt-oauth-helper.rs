use clap::Parser;
use serde::Deserialize;
use std::fs;
use yt_oauth::{OAuthConfig, start_auth_flow};

/// OAuth 2.0 helper tool for YouTube API authentication
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// OAuth client ID (or path to client_secret JSON file from Google Cloud Console)
    #[arg(long, required = true)]
    client_id: String,

    /// OAuth client secret (not required if using client_secret JSON file)
    #[arg(long)]
    client_secret: Option<String>,

    /// Path to save the OAuth token file
    #[arg(long, required = true)]
    token_path: String,
}

/// Structure for parsing Google Cloud OAuth client secret JSON file
#[derive(Deserialize, Debug)]
struct ClientSecretFile {
    installed: Option<InstalledApp>,
    web: Option<WebApp>,
}

#[derive(Deserialize, Debug)]
struct InstalledApp {
    client_id: String,
    client_secret: String,
}

#[derive(Deserialize, Debug)]
struct WebApp {
    client_id: String,
    client_secret: String,
}

/// Load client credentials from JSON file or use provided values
fn load_credentials(
    client_id_arg: &str,
    client_secret_arg: Option<&str>,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    // Check if client_id_arg is a file path
    if std::path::Path::new(client_id_arg).exists() {
        eprintln!("Loading OAuth credentials from file: {}", client_id_arg);

        let file_content = fs::read_to_string(client_id_arg).map_err(|e| {
            format!(
                "Failed to read client secret file '{}': {}",
                client_id_arg, e
            )
        })?;

        let client_secret_file: ClientSecretFile =
            serde_json::from_str(&file_content).map_err(|e| {
                format!(
                    "Failed to parse client secret JSON file '{}': {}",
                    client_id_arg, e
                )
            })?;

        // Try to get credentials from 'installed' field first, then 'web'
        if let Some(installed) = client_secret_file.installed {
            eprintln!("Using credentials from 'installed' application type");
            return Ok((installed.client_id, installed.client_secret));
        } else if let Some(web) = client_secret_file.web {
            eprintln!("Using credentials from 'web' application type");
            return Ok((web.client_id, web.client_secret));
        } else {
            return Err(
                "Client secret JSON file must contain either 'installed' or 'web' field".into(),
            );
        }
    }

    // Otherwise, use the provided client_id and client_secret arguments
    let client_secret = client_secret_arg
        .ok_or("--client-secret is required when --client-id is not a JSON file path")?;

    Ok((client_id_arg.to_string(), client_secret.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let (client_id, client_secret) =
        load_credentials(&args.client_id, args.client_secret.as_deref())?;

    eprintln!("Client ID: {}", client_id);

    let config = OAuthConfig::new(client_id, client_secret);

    // Start OAuth authorization flow
    let token = start_auth_flow(&config).await?;

    // Save token to file
    token.save_to_file(&args.token_path)?;

    eprintln!("\nOAuth token saved to: {}", args.token_path);
    eprintln!("You can now use this token with yt-comment-fetcher");

    Ok(())
}
