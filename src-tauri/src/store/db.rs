//! SQLite bootstrap and schema migrations (versioned via `user_version`).

use std::path::Path;

use rusqlite::Connection;

use crate::error::AppResult;

pub fn open_at(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS increments (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                jql        TEXT NOT NULL,
                start_date TEXT NOT NULL,
                end_date   TEXT NOT NULL,
                is_active  INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS epics (
                key               TEXT PRIMARY KEY,
                increment_id      INTEGER NOT NULL,
                name              TEXT NOT NULL,
                owner             TEXT,
                sp                REAL,
                start_date        TEXT,
                end_date          TEXT,
                status_category   TEXT NOT NULL,
                carried_from      TEXT,
                removed_from_plan INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS issues (
                key               TEXT PRIMARY KEY,
                epic_key          TEXT NOT NULL,
                summary           TEXT NOT NULL,
                sp                REAL,
                effective_sp      REAL NOT NULL,
                sp_imputed        INTEGER NOT NULL,
                status            TEXT NOT NULL,
                status_category   TEXT NOT NULL,
                resolution        TEXT,
                descoped          INTEGER NOT NULL,
                blocked           INTEGER NOT NULL,
                assignee          TEXT,
                created           TEXT NOT NULL,
                done_at           TEXT,
                reopened          INTEGER NOT NULL,
                current_sprint_id INTEGER,
                spill_count       INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_issues_epic ON issues(epic_key);
            CREATE TABLE IF NOT EXISTS sprints (
                id         INTEGER PRIMARY KEY,
                name       TEXT NOT NULL,
                state      TEXT NOT NULL,
                start_date TEXT,
                end_date   TEXT,
                board_id   INTEGER
            );
            CREATE TABLE IF NOT EXISTS issue_sprints (
                issue_key        TEXT NOT NULL,
                sprint_id        INTEGER NOT NULL,
                was_committed    INTEGER NOT NULL,
                added_mid_sprint INTEGER NOT NULL,
                done_at_close    INTEGER,
                PRIMARY KEY (issue_key, sprint_id)
            );
            CREATE TABLE IF NOT EXISTS status_events (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                issue_key     TEXT NOT NULL,
                at            TEXT NOT NULL,
                from_category TEXT NOT NULL,
                to_category   TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_status_events_issue ON status_events(issue_key);
            CREATE TABLE IF NOT EXISTS snapshots (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                increment_id   INTEGER NOT NULL,
                taken_at       TEXT NOT NULL,
                done_sp        REAL NOT NULL,
                scope_sp       REAL NOT NULL,
                in_progress_sp REAL NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sync_state (
                increment_id   INTEGER PRIMARY KEY,
                last_full_sync TEXT NOT NULL
            );
            PRAGMA user_version = 1;
            "#,
        )?;
    }
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query([key])?;
    Ok(match rows.next()? {
        Some(row) => Some(row.get(0)?),
        None => None,
    })
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}
