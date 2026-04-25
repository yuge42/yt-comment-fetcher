use crate::consumers::MessageConsumer;
use yt_grpc_client::LiveChatMessageListResponse;

/// Writes each message in a human-readable colored format to stderr.
///
/// Output format: `[cyan][AuthorName][/cyan] message text`
///
/// Writing to stderr keeps stdout clean for JSON output, so both
/// `--pretty-print` and JSON piping can be used at the same time.
pub struct PrettyConsumer;

impl MessageConsumer for PrettyConsumer {
    fn consume(
        &mut self,
        response: &LiveChatMessageListResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for item in &response.items {
            let name = item
                .author_details
                .as_ref()
                .and_then(|a| a.display_name.as_deref())
                .unwrap_or("unknown");
            let msg = item
                .snippet
                .as_ref()
                .and_then(|s| s.display_message.as_deref())
                .unwrap_or("");
            if !msg.is_empty() {
                eprintln!("\x1b[36m[{}]\x1b[0m {}", name, msg);
            }
        }
        Ok(())
    }
}
