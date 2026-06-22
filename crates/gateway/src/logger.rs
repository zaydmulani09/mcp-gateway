use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug)]
pub enum LoggerError {
    Sqlite(rusqlite::Error),
    Mutex,
}

impl From<rusqlite::Error> for LoggerError {
    fn from(e: rusqlite::Error) -> Self {
        LoggerError::Sqlite(e)
    }
}

impl std::fmt::Display for LoggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggerError::Sqlite(e) => write!(f, "sqlite error: {}", e),
            LoggerError::Mutex => write!(f, "mutex poisoned"),
        }
    }
}

impl std::error::Error for LoggerError {}

#[derive(Serialize)]
pub struct LogEntry {
    pub id: String,
    pub ts: String,
    pub server_name: String,
    pub client_ip: String,
    pub method: String,
    pub path: String,
    pub status: Option<i64>,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
}

pub struct RequestLogger {
    conn: Mutex<Connection>,
}

impl RequestLogger {
    pub fn new(db_path: &str) -> Result<RequestLogger, LoggerError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS request_log (
                id          TEXT PRIMARY KEY,
                ts          TEXT NOT NULL,
                server_name TEXT NOT NULL,
                client_ip   TEXT NOT NULL,
                method      TEXT NOT NULL,
                path        TEXT NOT NULL,
                status      INTEGER,
                latency_ms  INTEGER,
                error       TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_ts ON request_log(ts);
            CREATE INDEX IF NOT EXISTS idx_server ON request_log(server_name);",
        )?;
        Ok(RequestLogger {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert(&self, entry: &LogEntry) -> Result<(), LoggerError> {
        let conn = self.conn.lock().map_err(|_| LoggerError::Mutex)?;
        conn.execute(
            "INSERT INTO request_log
             (id, ts, server_name, client_ip, method, path, status, latency_ms, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id,
                entry.ts,
                entry.server_name,
                entry.client_ip,
                entry.method,
                entry.path,
                entry.status,
                entry.latency_ms,
                entry.error,
            ],
        )?;
        Ok(())
    }

    pub fn recent(&self, limit: u32) -> Result<Vec<LogEntry>, LoggerError> {
        let conn = self.conn.lock().map_err(|_| LoggerError::Mutex)?;
        let mut stmt = conn.prepare(
            "SELECT id, ts, server_name, client_ip, method, path, status, latency_ms, error
             FROM request_log
             ORDER BY ts DESC
             LIMIT ?1",
        )?;
        let entries = stmt
            .query_map(params![limit], |row| {
                Ok(LogEntry {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    server_name: row.get(2)?,
                    client_ip: row.get(3)?,
                    method: row.get(4)?,
                    path: row.get(5)?,
                    status: row.get(6)?,
                    latency_ms: row.get(7)?,
                    error: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(id: &str, ts: &str) -> LogEntry {
        LogEntry {
            id: id.to_string(),
            ts: ts.to_string(),
            server_name: "test-server".to_string(),
            client_ip: "127.0.0.1".to_string(),
            method: "GET".to_string(),
            path: "/tools/list".to_string(),
            status: Some(200),
            latency_ms: Some(42),
            error: None,
        }
    }

    #[test]
    fn db_creates_on_missing_path() {
        let result = RequestLogger::new(":memory:");
        assert!(result.is_ok());
    }

    #[test]
    fn insert_and_retrieve() {
        let logger = RequestLogger::new(":memory:").unwrap();
        let entry = sample_entry("abc-123", "2026-01-01T00:00:00Z");
        logger.insert(&entry).unwrap();

        let rows = logger.recent(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "abc-123");
        assert_eq!(rows[0].server_name, "test-server");
        assert_eq!(rows[0].method, "GET");
        assert_eq!(rows[0].path, "/tools/list");
        assert_eq!(rows[0].status, Some(200));
        assert_eq!(rows[0].latency_ms, Some(42));
        assert!(rows[0].error.is_none());
    }

    #[test]
    fn recent_respects_limit() {
        let logger = RequestLogger::new(":memory:").unwrap();
        for i in 0..5u8 {
            let entry = sample_entry(
                &format!("id-{}", i),
                &format!("2026-01-01T00:00:0{}Z", i),
            );
            logger.insert(&entry).unwrap();
        }

        let rows = logger.recent(3).unwrap();
        assert_eq!(rows.len(), 3);
        // ORDER BY ts DESC — most recent first
        assert_eq!(rows[0].id, "id-4");
    }
}
