use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

pub struct Db {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub raw_text: String,
    pub final_text: String,
    pub app: Option<String>,
    pub mode: String,
    pub asr_model: String,
    pub llm_model: String,
    pub duration_ms: i64,
    pub starred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictEntry {
    pub id: i64,
    pub from_text: String,
    pub to_text: String,
    pub note: Option<String>,
}

impl Db {
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        if let Some(p) = path.as_ref().parent() {
            std::fs::create_dir_all(p).ok();
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS history (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                raw_text TEXT NOT NULL,
                final_text TEXT NOT NULL,
                app TEXT,
                mode TEXT NOT NULL DEFAULT 'default',
                asr_model TEXT NOT NULL DEFAULT '',
                llm_model TEXT NOT NULL DEFAULT '',
                duration_ms INTEGER NOT NULL DEFAULT 0,
                starred INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_history_created ON history(created_at DESC);

            CREATE TABLE IF NOT EXISTS dictionary (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_text TEXT NOT NULL UNIQUE,
                to_text TEXT NOT NULL,
                note TEXT
            );

            CREATE TABLE IF NOT EXISTS prompt_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                system_prompt TEXT NOT NULL,
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS models (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL DEFAULT 0,
                sha256 TEXT,
                installed_at TEXT NOT NULL,
                UNIQUE(kind, name)
            );
        "#)?;
        Ok(())
    }

    pub fn insert_history(&self, r: &HistoryRecord) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT OR REPLACE INTO history (id,created_at,raw_text,final_text,app,mode,asr_model,llm_model,duration_ms,starred)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                r.id, r.created_at.to_rfc3339(), r.raw_text, r.final_text,
                r.app, r.mode, r.asr_model, r.llm_model, r.duration_ms, r.starred as i64
            ],
        )?;
        Ok(())
    }

    pub fn list_history(&self, limit: usize) -> anyhow::Result<Vec<HistoryRecord>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT id,created_at,raw_text,final_text,app,mode,asr_model,llm_model,duration_ms,starred
             FROM history ORDER BY created_at DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            let created_at: String = row.get(1)?;
            Ok(HistoryRecord {
                id: row.get(0)?,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                raw_text: row.get(2)?,
                final_text: row.get(3)?,
                app: row.get(4)?,
                mode: row.get(5)?,
                asr_model: row.get(6)?,
                llm_model: row.get(7)?,
                duration_ms: row.get(8)?,
                starred: row.get::<_, i64>(9)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn delete_history(&self, id: &str) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute("DELETE FROM history WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn dict_list(&self) -> anyhow::Result<Vec<DictEntry>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT id,from_text,to_text,note FROM dictionary ORDER BY id")?;
        let rows = stmt.query_map([], |row| Ok(DictEntry {
            id: row.get(0)?, from_text: row.get(1)?, to_text: row.get(2)?, note: row.get(3)?,
        }))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn dict_upsert(&self, from: &str, to: &str, note: Option<&str>) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO dictionary (from_text,to_text,note) VALUES (?1,?2,?3)
             ON CONFLICT(from_text) DO UPDATE SET to_text=excluded.to_text, note=excluded.note",
            params![from, to, note],
        )?;
        Ok(())
    }

    pub fn dict_delete(&self, id: i64) -> anyhow::Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute("DELETE FROM dictionary WHERE id=?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    #[test]
    fn db_history_roundtrip() {
        let p = std::env::temp_dir().join(format!("typeless-test-{}.db", Uuid::new_v4()));
        let db = Db::open(&p).unwrap();
        let r = HistoryRecord {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            raw_text: "hello".into(),
            final_text: "Hello.".into(),
            app: None, mode: "default".into(),
            asr_model: "base".into(), llm_model: "deepseek".into(),
            duration_ms: 1234, starred: false,
        };
        db.insert_history(&r).unwrap();
        let list = db.list_history(10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].final_text, "Hello.");
        std::fs::remove_file(p).ok();
    }
}
