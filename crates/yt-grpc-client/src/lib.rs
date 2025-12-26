pub mod youtube {
    pub mod api {
        pub mod v3 {
            tonic::include_proto!("youtube.api.v3");
        }
    }
}

pub use youtube::api::v3::*;

use tonic::transport::Channel;
use tonic::metadata::AsciiMetadataValue;

pub struct YouTubeClient {
    client: v3_data_live_chat_message_service_client::V3DataLiveChatMessageServiceClient<Channel>,
    api_key: Option<String>,
}

impl YouTubeClient {
    pub async fn connect(addr: String, api_key: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let client =
            v3_data_live_chat_message_service_client::V3DataLiveChatMessageServiceClient::connect(
                addr,
            )
            .await?;
        Ok(YouTubeClient { client, api_key })
    }

    pub async fn stream_comments(
        &mut self,
        live_chat_id: Option<String>,
    ) -> Result<tonic::Streaming<LiveChatMessageListResponse>, Box<dyn std::error::Error>> {
        let mut request = tonic::Request::new(LiveChatMessageListRequest {
            live_chat_id,
            hl: None,
            profile_image_size: None,
            max_results: None,
            page_token: None,
            part: vec!["snippet".to_string(), "authorDetails".to_string()],
        });

        // Add API key to metadata if provided
        if let Some(api_key) = &self.api_key {
            let metadata_value = AsciiMetadataValue::try_from(api_key.as_str())?;
            request.metadata_mut().insert("x-goog-api-key", metadata_value);
        }

        let response = self.client.stream_list(request).await?;
        Ok(response.into_inner())
    }
}
