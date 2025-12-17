pub mod youtube {
    pub mod api {
        pub mod v3 {
            tonic::include_proto!("youtube.api.v3");
        }
    }
}

pub use youtube::api::v3::*;

use tonic::transport::Channel;

pub struct YouTubeClient {
    client: v3_data_live_chat_message_service_client::V3DataLiveChatMessageServiceClient<Channel>,
}

impl YouTubeClient {
    pub async fn connect(addr: String) -> Result<Self, Box<dyn std::error::Error>> {
        let client =
            v3_data_live_chat_message_service_client::V3DataLiveChatMessageServiceClient::connect(
                addr,
            )
            .await?;
        Ok(YouTubeClient { client })
    }

    pub async fn stream_comments(
        &mut self,
        live_chat_id: Option<String>,
    ) -> Result<tonic::Streaming<LiveChatMessageListResponse>, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(LiveChatMessageListRequest {
            live_chat_id,
            hl: None,
            profile_image_size: None,
            max_results: None,
            page_token: None,
            part: vec!["snippet".to_string(), "authorDetails".to_string()],
        });

        let response = self.client.stream_list(request).await?;
        Ok(response.into_inner())
    }
}
