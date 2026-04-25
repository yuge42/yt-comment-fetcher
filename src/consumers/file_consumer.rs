use crate::consumers::MessageConsumer;
use std::fs::{File, OpenOptions};
use std::io::Write;
use yt_grpc_client::LiveChatMessageListResponse;

/// Writes each `LiveChatMessageListResponse` as a single JSON line (NDJSON) to a file.
pub struct FileConsumer {
    file: File,
}

impl FileConsumer {
    /// Open (or create and append to) the file at `path`.
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("Failed to open output file '{}': {}", path, e))?;
        Ok(Self { file })
    }
}

impl MessageConsumer for FileConsumer {
    fn consume(
        &mut self,
        response: &LiveChatMessageListResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if response.items.is_empty() {
            return Ok(());
        }
        let json = serde_json::to_string(response)?;
        writeln!(self.file, "{}", json)?;
        self.file.flush()?;
        Ok(())
    }
}
