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
        apply_v1(conn)?;
    }
    if version < 2 {
        apply_v2(conn)?;
    }
    Ok(())
}

fn apply_v1(conn: &Connection) -> AppResult<()> {
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
    Ok(())
}

/// Per-increment isolation: the cached Jira entities are now keyed by
/// (increment_id, key) so the same epic/issue can exist independently in
/// multiple increments and a sync of one increment can never touch another's
/// rows. The four affected tables are a pure cache (re-synced on demand), so we
/// drop and recreate them rather than migrate data; increments, snapshots,
/// sync_state and settings are preserved.
fn apply_v2(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
            DROP TABLE IF EXISTS issue_sprints;
            DROP TABLE IF EXISTS status_events;
            DROP TABLE IF EXISTS issues;
            DROP TABLE IF EXISTS epics;

            CREATE TABLE epics (
                key               TEXT NOT NULL,
                increment_id      INTEGER NOT NULL,
                name              TEXT NOT NULL,
                owner             TEXT,
                sp                REAL,
                start_date        TEXT,
                end_date          TEXT,
                status_category   TEXT NOT NULL,
                carried_from      TEXT,
                removed_from_plan INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (increment_id, key)
            );
            CREATE TABLE issues (
                key               TEXT NOT NULL,
                increment_id      INTEGER NOT NULL,
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
                spill_count       INTEGER NOT NULL,
                PRIMARY KEY (increment_id, key)
            );
            CREATE INDEX idx_issues_epic ON issues(increment_id, epic_key);
            CREATE TABLE issue_sprints (
                increment_id     INTEGER NOT NULL,
                issue_key        TEXT NOT NULL,
                sprint_id        INTEGER NOT NULL,
                was_committed    INTEGER NOT NULL,
                added_mid_sprint INTEGER NOT NULL,
                done_at_close    INTEGER,
                PRIMARY KEY (increment_id, issue_key, sprint_id)
            );
            CREATE TABLE status_events (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                increment_id  INTEGER NOT NULL,
                issue_key     TEXT NOT NULL,
                at            TEXT NOT NULL,
                from_category TEXT NOT NULL,
                to_category   TEXT NOT NULL
            );
            CREATE INDEX idx_status_events_issue ON status_events(increment_id, issue_key);
            PRAGMA user_version = 2;
            "#,
    )?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_lands_on_v2() {
        let conn = open_in_memory().unwrap();
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 2);
        // The v2 columns exist.
        conn.execute_batch("SELECT increment_id FROM issues; SELECT increment_id FROM issue_sprints; SELECT increment_id FROM status_events;").unwrap();
    }

    #[test]
    fn upgrades_v1_to_v2_preserving_non_cache_tables() {
        // Build a v1 database with old-schema cache data plus the tables that
        // must survive the upgrade (settings, snapshots).
        let conn = Connection::open_in_memory().unwrap();
        apply_v1(&conn).unwrap();
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 1);
        set_setting(&conn, "token", "keep-me").unwrap();
        conn.execute(
            "INSERT INTO snapshots(increment_id, taken_at, done_sp, scope_sp, in_progress_sp) VALUES (1, '2026-01-01T00:00:00+00:00', 1.0, 2.0, 0.5)",
            [],
        ).unwrap();
        // Old-schema row (issues had no increment_id at v1).
        conn.execute(
            "INSERT INTO issues(key, epic_key, summary, sp, effective_sp, sp_imputed, status, status_category, descoped, blocked, created, reopened, spill_count)
             VALUES ('OLD-1','E-1','x',1.0,1.0,0,'Done','done',0,0,'2026-01-01T00:00:00+00:00',0,0)",
            [],
        ).unwrap();

        // Run the real migration path.
        migrate(&conn).unwrap();

        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 2);
        // Preserved tables keep their data.
        assert_eq!(get_setting(&conn, "token").unwrap().as_deref(), Some("keep-me"));
        let snaps: i64 = conn.query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0)).unwrap();
        assert_eq!(snaps, 1);
        // Cache tables were rebuilt on the new schema (old rows dropped, new column present).
        let issues: i64 = conn.query_row("SELECT COUNT(*) FROM issues", [], |r| r.get(0)).unwrap();
        assert_eq!(issues, 0);
        conn.execute_batch("SELECT increment_id, epic_key FROM issues;").unwrap();
    }
}
