use crate::consumers::MessageConsumer;
use rusqlite::{Connection, params};
use yt_grpc_client::LiveChatMessageListResponse;

/// Stores live chat messages in a SQLite database.
///
/// Schema:
/// - `authors` – one row per unique channel, updated on every encounter.
/// - `messages` – one row per live chat message item; includes `next_page_token`
///   from the response batch so the database can be used with `--resume`.
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
                raw_json          TEXT NOT NULL,
                next_page_token   TEXT
            );",
        )
        .map_err(|e| format!("Failed to initialise SQLite schema: {}", e))?;

        // Migrate databases created before next_page_token was added to messages.
        // ALTER TABLE fails if the column already exists; that error is intentionally ignored.
        let _ = conn.execute("ALTER TABLE messages ADD COLUMN next_page_token TEXT", []);

        Ok(Self { conn })
    }

    /// Read the last known `live_chat_id` and `next_page_token` from the most
    /// recently inserted message in the database.
    ///
    /// Returns `(chat_id, page_token)` – either or both may be `None` if the
    /// database is empty or the messages predate the `next_page_token` column.
    pub fn read_resume_info(
        path: &str,
    ) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error>> {
        let conn = Connection::open(path)
            .map_err(|e| format!("Failed to open SQLite database '{}': {}", path, e))?;

        let result = conn.query_row(
            "SELECT live_chat_id, next_page_token FROM messages ORDER BY rowid DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        );

        match result {
            Ok((chat_id, page_token)) => Ok((chat_id, page_token)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok((None, None)),
            Err(e) => Err(e.into()),
        }
    }
}

impl MessageConsumer for SqliteConsumer {
    fn consume(
        &mut self,
        response: &LiveChatMessageListResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if response.items.is_empty() {
            return Ok(());
        }

        let tx = self.conn.transaction()?;

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

            // Store next_page_token alongside each message so the database can be
            // used with --resume: querying the last row gives both chat ID and token.
            tx.execute(
                "INSERT OR IGNORE INTO messages
                    (id, live_chat_id, author_channel_id, published_at, type,
                     display_message, raw_json, next_page_token)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    message_id,
                    live_chat_id,
                    author_channel_id,
                    published_at,
                    msg_type,
                    display_message,
                    raw_json,
                    response.next_page_token.as_deref(),
                ],
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
        // Verify both tables exist
        let count: i64 = consumer
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('authors','messages')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "both tables should be created");
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
    fn test_consume_stores_next_page_token_on_message() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let msg = make_message("msg-tok", "chan-tok", "Tokuser", "token test");
        let mut response = make_response(vec![msg]);
        response.next_page_token = Some("token-abc".to_string());

        consumer.consume(&response).expect("consume");

        let token: Option<String> = consumer
            .conn
            .query_row(
                "SELECT next_page_token FROM messages WHERE id = 'msg-tok'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(token.as_deref(), Some("token-abc"));
    }

    #[test]
    fn test_consume_updates_next_page_token_on_subsequent_responses() {
        let mut consumer = SqliteConsumer::open(":memory:").expect("open should succeed");

        let msg1 = make_message("msg-a", "chan-a", "Alice", "first");
        let mut resp1 = make_response(vec![msg1]);
        resp1.next_page_token = Some("token-1".to_string());
        consumer.consume(&resp1).expect("first consume");

        let msg2 = make_message("msg-b", "chan-b", "Bob", "second");
        let mut resp2 = make_response(vec![msg2]);
        resp2.next_page_token = Some("token-2".to_string());
        consumer.consume(&resp2).expect("second consume");

        // The most recent message (msg-b) should carry the latest token.
        let token: Option<String> = consumer
            .conn
            .query_row(
                "SELECT next_page_token FROM messages ORDER BY rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            token.as_deref(),
            Some("token-2"),
            "latest message should carry the latest token"
        );
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
