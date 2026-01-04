use clap::Parser;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use yt_oauth::{OAUTH_CALLBACK_PORT, OAuthConfig, OAuthToken};

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

/// Generate PKCE verifier and challenge
fn generate_pkce() -> (String, String) {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use rand::Rng;
    use rand::distributions::Alphanumeric;
    use sha2::{Digest, Sha256};

    // Generate random verifier (43-128 characters) using cryptographically secure RNG
    let verifier: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    // Generate challenge: base64url(SHA256(verifier))
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(hash);

    (verifier, challenge)
}

/// Generate authorization URL
fn generate_auth_url(config: &OAuthConfig) -> (String, String) {
    let (verifier, challenge) = generate_pkce();

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
        client_id={}&\
        redirect_uri={}&\
        response_type=code&\
        scope={}&\
        code_challenge={}&\
        code_challenge_method=S256&\
        access_type=offline&\
        prompt=consent",
        urlencoding::encode(&config.client_id),
        urlencoding::encode(&config.redirect_uri),
        urlencoding::encode(&config.scope),
        urlencoding::encode(&challenge),
    );

    (auth_url, verifier)
}

/// Exchange authorization code for tokens
async fn exchange_code(
    config: &OAuthConfig,
    code: &str,
    verifier: &str,
) -> Result<OAuthToken, Box<dyn std::error::Error>> {
    eprintln!("Exchanging authorization code for tokens...");

    let client = reqwest::Client::new();
    let params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("code", code),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", config.redirect_uri.as_str()),
    ];

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await?;
        return Err(format!(
            "Failed to exchange authorization code (status {}): {}",
            status, body
        )
        .into());
    }

    let token_response: serde_json::Value = response.json().await?;

    let access_token = token_response
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing access_token in token response")?
        .to_string();

    let refresh_token = token_response
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing refresh_token in token response")?
        .to_string();

    let expires_in = token_response
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .ok_or("Missing expires_in in token response")?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let token = OAuthToken {
        access_token,
        refresh_token,
        token_type: token_response
            .get("token_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Bearer")
            .to_string(),
        expires_at: now + expires_in,
    };

    eprintln!("Successfully obtained OAuth tokens");

    Ok(token)
}

/// Start OAuth flow with local callback server
async fn start_auth_flow(config: &OAuthConfig) -> Result<OAuthToken, Box<dyn std::error::Error>> {
    let (auth_url, verifier) = generate_auth_url(config);

    eprintln!("\n=================================================");
    eprintln!("OAuth 2.0 Authorization Required");
    eprintln!("=================================================");
    eprintln!("\nPlease visit the following URL to authorize the application:\n");
    eprintln!("{}\n", auth_url);
    eprintln!("Waiting for authorization...");
    eprintln!("=================================================\n");

    // Shared state for callback
    let code_receiver = Arc::new(Mutex::new(None::<String>));
    let code_receiver_clone = code_receiver.clone();

    // Create callback handler
    use axum::{
        Router,
        extract::Query,
        response::{Html, IntoResponse},
        routing::get,
    };
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct AuthCallback {
        code: Option<String>,
        error: Option<String>,
    }

    let callback_handler = move |Query(params): Query<AuthCallback>| async move {
        if let Some(error) = params.error {
            return Html(format!(
                "<html><body><h1>Authorization Failed</h1><p>Error: {}</p>\
                <p>You can close this window.</p></body></html>",
                error
            ))
            .into_response();
        }

        if let Some(code) = params.code {
            *code_receiver_clone.lock().await = Some(code);
            return Html(
                "<html><body><h1>Authorization Successful!</h1>\
                <p>You can close this window and return to the application.</p></body></html>",
            )
            .into_response();
        }

        Html("<html><body><h1>Authorization Failed</h1><p>No code received</p></body></html>")
            .into_response()
    };

    let app = Router::new().route("/oauth2callback", get(callback_handler));

    // Start server
    let listener =
        tokio::net::TcpListener::bind(format!("127.0.0.1:{}", OAUTH_CALLBACK_PORT)).await?;
    let server = axum::serve(listener, app);

    // Run server until we get a code
    let server_handle = tokio::spawn(async move {
        server.await.ok();
    });

    // Wait for authorization code (with timeout)
    let timeout = tokio::time::Duration::from_secs(300); // 5 minutes
    let start = tokio::time::Instant::now();

    let code = loop {
        if start.elapsed() > timeout {
            return Err("OAuth authorization timeout (5 minutes)".into());
        }

        let code_opt = code_receiver.lock().await.clone();
        if let Some(code) = code_opt {
            break code;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    };

    // Stop server
    server_handle.abort();

    // Exchange code for tokens
    exchange_code(config, &code, &verifier).await
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
