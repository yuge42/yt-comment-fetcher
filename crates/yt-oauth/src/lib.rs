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
