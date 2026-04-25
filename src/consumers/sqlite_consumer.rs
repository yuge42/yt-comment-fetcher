use crate::consumers::MessageConsumer;
use rusqlite::{Connection, params};
use yt_grpc_client::LiveChatMessageListResponse;

/// Stores live chat messages in a SQLite database.
///
/// Schema:
/// - `authors` – one row per unique channel, updated on every encounter.
/// - `messages` – one row per live chat message item.
pub struct SqliteConsumer {
    conn: Connection,
}

impl SqliteConsumer {
    /// Open (or create) the SQLite database at `path` and initialise the schema.
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)
            .map_err(|e| format!("Failed to open SQLite database '{}': {}", path, e))?;

        conn.execute_batch(
            // WAL (Write-Ahead Logging) journal mode offers better concurrency:
            // readers never block writers and writers never block readers, which
            // suits the live-streaming use-case where messages arrive continuously.
            "PRAGMA journal_mode=WAL;

            CREATE TABLE IF NOT EXISTS authors (
                channel_id        TEXT PRIMARY KEY,
                channel_url       TEXT,
                display_name      TEXT,
                profile_image_url TEXT,
                is_verified       INTEGER,
                is_chat_owner     INTEGER,
                is_chat_sponsor   INTEGER,
                is_chat_moderator INTEGER
            );

            CREATE TABLE IF NOT EXISTS messages (
                id                TEXT PRIMARY KEY,
                live_chat_id      TEXT,
                author_channel_id TEXT REFERENCES authors(channel_id),
                published_at      TEXT,
                type              INTEGER,
                display_message   TEXT,
                raw_json          TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS metadata (
                key   TEXT PRIMARY KEY,
                value TEXT
            );",
        )
        .map_err(|e| format!("Failed to initialise SQLite schema: {}", e))?;

        Ok(Self { conn })
    }

    /// Read the last known `live_chat_id` and `next_page_token` stored in the database.
    ///
    /// Returns `(chat_id, page_token)` – either or both may be `None` if the database is
    /// empty or was created with an older schema that lacks the `metadata` table.
    pub fn read_resume_info(
        path: &str,
    ) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error>> {
        let conn = Connection::open(path)
            .map_err(|e| format!("Failed to open SQLite database '{}': {}", path, e))?;

        // The metadata table may not exist (older databases without the schema).
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='metadata'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !table_exists {
            return Ok((None, None));
        }

        let chat_id: Option<String> = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'live_chat_id'",
                [],
                |row| row.get(0),
            )
            .ok();

        let next_page_token: Option<String> = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'next_page_token'",
                [],
                |row| row.get(0),
            )
            .ok();

        Ok((chat_id, next_page_token))
    }
}

impl MessageConsumer for SqliteConsumer {
    fn consume(
        &mut self,
        response: &LiveChatMessageListResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Open a transaction that covers both message/author inserts and metadata updates
        // so resume state is always consistent with the persisted messages.
        let tx = self.conn.transaction()?;

        // Always persist next_page_token for resume support, even when the response
        // contains no items (e.g. end of a page with an empty last batch).
        if let Some(page_token) = response.next_page_token.as_deref() {
            tx.execute(
                "INSERT INTO metadata (key, value) VALUES ('next_page_token', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![page_token],
            )?;
        }

        if response.items.is_empty() {
            tx.commit()?;
            return Ok(());
        }

        for item in &response.items {
            // --- author details ---
            let author_channel_id: Option<&str> = item
                .author_details
                .as_ref()
                .and_then(|a| a.channel_id.as_deref());

            if let Some(author) = &item.author_details {
                if let Some(channel_id) = &author.channel_id {
                    tx.execute(
                        "INSERT INTO authors
                            (channel_id, channel_url, display_name, profile_image_url,
                             is_verified, is_chat_owner, is_chat_sponsor, is_chat_moderator)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                         ON CONFLICT(channel_id) DO UPDATE SET
                            channel_url       = excluded.channel_url,
                            display_name      = excluded.display_name,
                            profile_image_url = excluded.profile_image_url,
                            is_verified       = excluded.is_verified,
                            is_chat_owner     = excluded.is_chat_owner,
                            is_chat_sponsor   = excluded.is_chat_sponsor,
                            is_chat_moderator = excluded.is_chat_moderator",
                        params![
                            channel_id,
                            author.channel_url.as_deref(),
                            author.display_name.as_deref(),
                            author.profile_image_url.as_deref(),
                            author.is_verified.map(|v| v as i32),
                            author.is_chat_owner.map(|v| v as i32),
                            author.is_chat_sponsor.map(|v| v as i32),
                            author.is_chat_moderator.map(|v| v as i32),
                        ],
                    )?;
                }
            }

            // --- message ---
            let message_id = match &item.id {
                Some(id) => id.as_str(),
                None => continue, // skip messages without an ID
            };

            let raw_json = serde_json::to_string(item)?;

            let (live_chat_id, published_at, msg_type, display_message) =
                if let Some(snippet) = &item.snippet {
                    (
                        snippet.live_chat_id.as_deref(),
                        snippet.published_at.as_deref(),
                        snippet.r#type,
                        snippet.display_message.as_deref(),
                    )
                } else {
                    (None, None, None, None)
                };

            tx.execute(
                "INSERT OR IGNORE INTO messages
                    (id, live_chat_id, author_channel_id, published_at, type, display_message, raw_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    message_id,
                    live_chat_id,
                    author_channel_id,
                    published_at,
                    msg_type,
                    display_message,
                    raw_json,
                ],
            )?;
        }

        // Persist live_chat_id for resume support.
        // All items in a batch share the same chat, so the first item's snippet is sufficient.
        let live_chat_id = response
            .items
            .first()
            .and_then(|item| item.snippet.as_ref())
            .and_then(|s| s.live_chat_id.as_deref());

        if let Some(chat_id) = live_chat_id {
            tx.execute(
                "INSERT INTO metadata (key, value) VALUES ('live_chat_id', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![chat_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yt_grpc_client::{
        LiveChatMessage, LiveChatMessageAuthorDetails, LiveChatMessageListResponse,
        LiveChatMessageSnippet,
    };

    fn make_response(items: Vec<LiveChatMessage>) -> LiveChatMessageListResponse {
        LiveChatMessageListResponse {
            items,
            ..Default::default()
        }
    }

    fn make_message(
        id: &str,
        channel_id: &str,
        display_name: &str,
        display_message: &str,
    ) -> LiveChatMessage {
        LiveChatMessage {
            id: Some(id.to_string()),
            snippet: Some(LiveChatMessageSnippet {
                live_chat_id: Some("chat-123".to_string()),
                display_message: Some(display_message.to_string()),
                ..Default::default()
            }),
            author_details: Some(LiveChatMessageAuthorDetails {
                channel_id: Some(channel_id.to_string()),
                display_name: Some(display_name.to_string()),
                is_chat_owner: Some(false),
                is_chat_moderator: Some(false),
                is_chat_sponsor: Some(false),
                is_verified: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_open_in_memory_creates_schema() {
        let consumer = SqliteConsumer::open(":memory:").expect("open should succeed");
        // Verify all three tables exist
        let count: i64 = consumer
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('authors','messages','metadata')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3, "all three tables should be created");
    }

    #[test]
    fn test_consume_inserts_author_and_message() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let msg = make_message("msg-1", "chan-abc", "Alice", "Hello!");
        let response = make_response(vec![msg]);

        consumer.consume(&response).expect("consume should succeed");

        let author_count: i64 = consumer
            .conn
            .query_row("SELECT COUNT(*) FROM authors", [], |row| row.get(0))
            .unwrap();
        assert_eq!(author_count, 1);

        let msg_count: i64 = consumer
            .conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(msg_count, 1);
    }

    #[test]
    fn test_consume_skips_empty_response() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");
        let response = make_response(vec![]);
        consumer
            .consume(&response)
            .expect("empty consume should succeed");

        let count: i64 = consumer
            .conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_consume_deduplicates_messages() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let msg = make_message("msg-dup", "chan-xyz", "Bob", "Dup!");
        let response = make_response(vec![msg]);

        consumer.consume(&response.clone()).expect("first consume");
        consumer.consume(&response).expect("second consume");

        let count: i64 = consumer
            .conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "duplicate messages must not be double-inserted");
    }

    #[test]
    fn test_consume_updates_author_on_conflict() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let msg1 = make_message("msg-1", "chan-1", "OriginalName", "Hi");
        consumer
            .consume(&make_response(vec![msg1]))
            .expect("first insert");

        let msg2 = make_message("msg-2", "chan-1", "UpdatedName", "Hey");
        consumer
            .consume(&make_response(vec![msg2]))
            .expect("upsert");

        let name: String = consumer
            .conn
            .query_row(
                "SELECT display_name FROM authors WHERE channel_id = 'chan-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "UpdatedName", "author display_name should be updated");
    }

    #[test]
    fn test_consume_stores_raw_json() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let msg = make_message("msg-json", "chan-j", "JsonUser", "json test");
        consumer
            .consume(&make_response(vec![msg]))
            .expect("consume");

        let raw: String = consumer
            .conn
            .query_row(
                "SELECT raw_json FROM messages WHERE id = 'msg-json'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        // raw_json should be valid JSON containing the message ID
        let parsed: serde_json::Value =
            serde_json::from_str(&raw).expect("raw_json should be valid JSON");
        assert_eq!(parsed["id"], "msg-json");
    }

    #[test]
    fn test_consume_persists_metadata() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let msg = make_message("msg-meta", "chan-meta", "Meta", "metadata test");
        let mut response = make_response(vec![msg]);
        response.next_page_token = Some("token-abc".to_string());

        consumer.consume(&response).expect("consume");

        let chat_id: String = consumer
            .conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'live_chat_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(chat_id, "chat-123");

        let page_token: String = consumer
            .conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'next_page_token'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(page_token, "token-abc");
    }

    #[test]
    fn test_consume_updates_metadata_on_subsequent_responses() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let msg1 = make_message("msg-a", "chan-a", "Alice", "first");
        let mut resp1 = make_response(vec![msg1]);
        resp1.next_page_token = Some("token-1".to_string());
        consumer.consume(&resp1).expect("first consume");

        let msg2 = make_message("msg-b", "chan-b", "Bob", "second");
        let mut resp2 = make_response(vec![msg2]);
        resp2.next_page_token = Some("token-2".to_string());
        consumer.consume(&resp2).expect("second consume");

        let page_token: String = consumer
            .conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'next_page_token'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            page_token, "token-2",
            "metadata should reflect the latest token"
        );
    }

    #[test]
    fn test_consume_persists_page_token_for_empty_response() {
        // next_page_token should be stored even when items is empty.
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let mut response = make_response(vec![]);
        response.next_page_token = Some("token-empty".to_string());
        consumer.consume(&response).expect("consume empty response");

        let page_token: String = consumer
            .conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'next_page_token'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(page_token, "token-empty");
    }

    #[test]
    fn test_read_resume_info_from_file() {
        // Write to a real temp file so read_resume_info (which opens its own connection) can read it.
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("resume_test.db");
        let db_path_str = db_path.to_str().unwrap();

        let mut consumer = SqliteConsumer::open(db_path_str).expect("open");

        let msg = make_message("msg-r", "chan-r", "Resume", "hi");
        let mut response = make_response(vec![msg]);
        response.next_page_token = Some("token-resume".to_string());
        consumer.consume(&response).expect("consume");

        // Drop the consumer to close its connection before read_resume_info opens another.
        drop(consumer);

        let (chat_id, page_token) =
            SqliteConsumer::read_resume_info(db_path_str).expect("read_resume_info");
        assert_eq!(chat_id.as_deref(), Some("chat-123"));
        assert_eq!(page_token.as_deref(), Some("token-resume"));
    }

    #[test]
    fn test_read_resume_info_empty_db() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("empty.db");
        let db_path_str = db_path.to_str().unwrap();

        // Create schema but write nothing
        let _consumer = SqliteConsumer::open(db_path_str).expect("open");
        drop(_consumer);

        let (chat_id, page_token) =
            SqliteConsumer::read_resume_info(db_path_str).expect("read_resume_info");
        assert!(chat_id.is_none());
        assert!(page_token.is_none());
    }
}
