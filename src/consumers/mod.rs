pub mod file_consumer;
pub mod sqlite_consumer;
pub mod stdout_consumer;

use yt_grpc_client::LiveChatMessageListResponse;

/// Trait for consumers that process live chat message responses.
pub trait MessageConsumer {
    /// Handle a single batch of live chat messages.
    fn consume(
        &mut self,
        response: &LiveChatMessageListResponse,
    ) -> Result<(), Box<dyn std::error::Error>>;
}
