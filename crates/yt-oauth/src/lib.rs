use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default OAuth callback port
pub const OAUTH_CALLBACK_PORT: u16 = 8080;

/// OAuth 2.0 token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Access token for API requests
    pub access_token: String,
    /// Refresh token for getting new access tokens
    pub refresh_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expiry time as Unix timestamp (seconds since epoch)
    pub expires_at: u64,
}

impl OAuthToken {
    /// Check if the token is expired or will expire soon (within 60 seconds)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        // Consider token expired if it expires within 60 seconds
        now + 60 >= self.expires_at
    }

    /// Load token from file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read OAuth token file '{}': {}", path, e))?;
        let token: OAuthToken = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse OAuth token file '{}': {}", path, e))?;
        Ok(token)
    }

    /// Save token to file with secure permissions
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;

        // Write the file
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write OAuth token file '{}': {}", path, e))?;

        // Set secure permissions (owner read/write only) on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, permissions).map_err(|e| {
                format!("Failed to set permissions on token file '{}': {}", path, e)
            })?;
        }

        Ok(())
    }
}

/// OAuth configuration
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
    /// Redirect URI for OAuth callback
    pub redirect_uri: String,
    /// OAuth scope(s)
    pub scope: String,
}

impl OAuthConfig {
    /// Create new OAuth configuration with YouTube defaults
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri: format!("http://localhost:{}/oauth2callback", OAUTH_CALLBACK_PORT),
            scope: "https://www.googleapis.com/auth/youtube.force-ssl".to_string(),
        }
    }
}

/// OAuth manager handles token refresh
pub struct OAuthManager {
    config: OAuthConfig,
    token: Option<OAuthToken>,
}

impl OAuthManager {
    /// Create new OAuth manager
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            config,
            token: None,
        }
    }

    /// Load token from file
    pub fn load_token(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.token = Some(OAuthToken::load_from_file(path)?);
        Ok(())
    }

    /// Get valid access token, refreshing if necessary
    pub async fn get_access_token(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("No OAuth token loaded")?;

        if token.is_expired() {
            eprintln!("Access token expired, refreshing...");
            self.refresh_token().await?;
        }

        Ok(self
            .token
            .as_ref()
            .expect("Token should exist after refresh")
            .access_token
            .clone())
    }

    /// Refresh the access token using the refresh token
    async fn refresh_token(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_token = self.token.as_ref().ok_or("No OAuth token loaded")?;

        eprintln!("Refreshing OAuth token...");

        let client = reqwest::Client::new();
        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
            ("refresh_token", current_token.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
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
                "Failed to refresh OAuth token (status {}): {}",
                status, body
            )
            .into());
        }

        let refresh_response: serde_json::Value = response.json().await?;

        // Update the token with new access token
        let new_access_token = refresh_response
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or("Missing access_token in refresh response")?
            .to_string();

        let expires_in = refresh_response
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .ok_or("Missing expires_in in refresh response")?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        // Create updated token
        let updated_token = OAuthToken {
            access_token: new_access_token,
            refresh_token: current_token.refresh_token.clone(), // Keep existing refresh token
            token_type: refresh_response
                .get("token_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Bearer")
                .to_string(),
            expires_at: now + expires_in,
        };

        self.token = Some(updated_token);

        eprintln!("OAuth token refreshed successfully");

        Ok(())
    }

    /// Save current token to file
    pub fn save_token(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("No OAuth token to save")?;
        token.save_to_file(path)
    }
}

/// Generate PKCE verifier and challenge
pub fn generate_pkce() -> (String, String) {
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
pub fn generate_auth_url(config: &OAuthConfig) -> (String, String) {
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
pub async fn exchange_code(
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
pub async fn start_auth_flow(
    config: &OAuthConfig,
) -> Result<OAuthToken, Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use tokio::sync::Mutex;

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
