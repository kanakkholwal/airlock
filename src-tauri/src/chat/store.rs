use anyhow::Result;
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;

/// Decision (architecture): messages hold typed parts so tool calls, reasoning and media
/// (Stage 4) need no schema migration. Provenance: Ensu `db/` chat schema, re-implemented.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    Text {
        text: String,
    },
    Reasoning {
        text: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        arguments: String,
    },
    ToolResult {
        call_id: String,
        output: String,
        is_error: bool,
    },
    Image {
        path: String,
    },
    Audio {
        path: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    fn as_str(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }

    fn parse(s: &str) -> Role {
        match s {
            "system" => Role::System,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => Role::User,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub model: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: Role,
    pub parts: Vec<Part>,
    pub created_at: i64,
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(
        "CREATE TABLE conversations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            model TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX messages_by_conversation ON messages(conversation_id, id);
        CREATE TABLE parts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            idx INTEGER NOT NULL,
            payload TEXT NOT NULL
        );
        CREATE INDEX parts_by_message ON parts(message_id, idx);",
    )])
}

pub struct ChatStore {
    conn: Connection,
}

impl ChatStore {
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        migrations().to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn create_conversation(&self, title: &str, model: Option<&str>) -> Result<i64> {
        let now = now();
        self.conn.execute(
            "INSERT INTO conversations (title, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![title, model, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn append_message(
        &mut self,
        conversation_id: i64,
        role: Role,
        parts: &[Part],
    ) -> Result<i64> {
        let now = now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO messages (conversation_id, role, created_at) VALUES (?1, ?2, ?3)",
            params![conversation_id, role.as_str(), now],
        )?;
        let message_id = tx.last_insert_rowid();
        for (idx, part) in parts.iter().enumerate() {
            tx.execute(
                "INSERT INTO parts (message_id, idx, payload) VALUES (?1, ?2, ?3)",
                params![message_id, idx as i64, serde_json::to_string(part)?],
            )?;
        }
        tx.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![now, conversation_id],
        )?;
        tx.commit()?;
        Ok(message_id)
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, model, updated_at FROM conversations ORDER BY updated_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Conversation {
                id: r.get(0)?,
                title: r.get(1)?,
                model: r.get(2)?,
                updated_at: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn load_messages(&self, conversation_id: i64) -> Result<Vec<Message>> {
        let mut msgs = self.conn.prepare(
            "SELECT id, role, created_at FROM messages WHERE conversation_id = ?1 ORDER BY id",
        )?;
        let mut parts = self
            .conn
            .prepare("SELECT payload FROM parts WHERE message_id = ?1 ORDER BY idx")?;
        let heads = msgs
            .query_map(params![conversation_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out = Vec::with_capacity(heads.len());
        for (id, role, created_at) in heads {
            let payloads = parts
                .query_map(params![id], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let parts = payloads
                .iter()
                .map(|p| serde_json::from_str(p))
                .collect::<std::result::Result<Vec<Part>, _>>()?;
            out.push(Message {
                id,
                conversation_id,
                role: Role::parse(&role),
                parts,
                created_at,
            });
        }
        Ok(out)
    }

    /// Removes the newest message, used when a reply is regenerated.
    pub fn delete_last_message(&self, conversation_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM messages WHERE id = (SELECT MAX(id) FROM messages WHERE conversation_id = ?1)",
            params![conversation_id],
        )?;
        Ok(())
    }

    pub fn rename_conversation(&self, id: i64, title: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE conversations SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    pub fn delete_conversation(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_typed_parts_in_order() {
        let mut s = ChatStore::in_memory().unwrap();
        let c = s.create_conversation("t", Some("lfm3")).unwrap();
        let parts = vec![
            Part::Reasoning {
                text: "think".into(),
            },
            Part::ToolCall {
                call_id: "1".into(),
                name: "calc".into(),
                arguments: "{}".into(),
            },
            Part::Text {
                text: "done".into(),
            },
        ];
        s.append_message(c, Role::Assistant, &parts).unwrap();
        s.append_message(
            c,
            Role::Tool,
            &[Part::ToolResult {
                call_id: "1".into(),
                output: "4".into(),
                is_error: false,
            }],
        )
        .unwrap();
        let got = s.load_messages(c).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].parts, parts);
        assert_eq!(got[1].role, Role::Tool);
    }

    #[test]
    fn deleting_the_last_message_removes_its_parts_only() {
        let mut s = ChatStore::in_memory().unwrap();
        let c = s.create_conversation("t", None).unwrap();
        s.append_message(c, Role::User, &[Part::Text { text: "q".into() }])
            .unwrap();
        s.append_message(c, Role::Assistant, &[Part::Text { text: "a".into() }])
            .unwrap();
        s.delete_last_message(c).unwrap();
        let left = s.load_messages(c).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].role, Role::User);
    }

    #[test]
    fn deleting_a_conversation_cascades() {
        let mut s = ChatStore::in_memory().unwrap();
        let c = s.create_conversation("t", None).unwrap();
        s.append_message(c, Role::User, &[Part::Text { text: "hi".into() }])
            .unwrap();
        s.delete_conversation(c).unwrap();
        assert!(s.list_conversations().unwrap().is_empty());
        let orphans: i64 = s
            .conn
            .query_row("SELECT COUNT(*) FROM parts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(orphans, 0);
    }
}
