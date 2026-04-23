use crate::consumers::MessageConsumer;
use yt_grpc_client::LiveChatMessageListResponse;

/// Writes each `LiveChatMessageListResponse` as a single JSON line to stdout.
pub struct StdoutConsumer;

impl MessageConsumer for StdoutConsumer {
    fn consume(
        &mut self,
        response: &LiveChatMessageListResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if response.items.is_empty() {
            return Ok(());
        }
        let json = serde_json::to_string(response)?;
        println!("{}", json);
        Ok(())
    }
}
