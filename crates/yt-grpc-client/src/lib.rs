pub mod youtube {
    pub mod api {
        pub mod v3 {
            tonic::include_proto!("youtube.api.v3");
        }
    }
}

pub use youtube::api::v3::*;

use tonic::metadata::AsciiMetadataValue;
use tonic::transport::Channel;

pub struct YouTubeClient {
    client: v3_data_live_chat_message_service_client::V3DataLiveChatMessageServiceClient<Channel>,
    api_key: Option<String>,
    oauth_token: Option<String>,
}

impl YouTubeClient {
    pub async fn connect(
        addr: String,
        api_key: Option<String>,
        oauth_token: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let client =
            v3_data_live_chat_message_service_client::V3DataLiveChatMessageServiceClient::connect(
                addr,
            )
            .await?;
        Ok(YouTubeClient {
            client,
            api_key,
            oauth_token,
        })
    }

    pub async fn stream_comments(
        &mut self,
        live_chat_id: Option<String>,
        page_token: Option<String>,
    ) -> Result<tonic::Streaming<LiveChatMessageListResponse>, Box<dyn std::error::Error>> {
        let mut request = tonic::Request::new(LiveChatMessageListRequest {
            live_chat_id,
            hl: None,
            profile_image_size: None,
            max_results: None,
            page_token,
            part: vec!["snippet".to_string(), "authorDetails".to_string()],
        });

        // Add authentication to metadata
        if let Some(api_key) = &self.api_key {
            // API key authentication
            let metadata_value = AsciiMetadataValue::try_from(api_key.as_str())?;
            request
                .metadata_mut()
                .insert("x-goog-api-key", metadata_value);
        } else if let Some(oauth_token) = &self.oauth_token {
            // OAuth token authentication
            let auth_header = format!("Bearer {}", oauth_token);
            let metadata_value = AsciiMetadataValue::try_from(auth_header.as_str())?;
            request
                .metadata_mut()
                .insert("authorization", metadata_value);
        }

        let response = self.client.stream_list(request).await?;
        Ok(response.into_inner())
    }
}
