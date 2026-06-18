//! SQLite cache for synced Jira data. Write path: replace the increment's
//! issues/links/events wholesale per sync (a sync is the unit of
//! consistency). Read path: reconstruct domain types for the query layer.

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection};

use crate::domain::model::*;
use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Increments
// ---------------------------------------------------------------------------

pub fn list_increments(conn: &Connection) -> AppResult<Vec<Increment>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, jql, start_date, end_date, is_active FROM increments ORDER BY start_date DESC",
    )?;
    let rows = stmt.query_map([], row_to_increment)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_increment(conn: &Connection, id: i64) -> AppResult<Increment> {
    let mut stmt = conn.prepare(
        "SELECT id, name, jql, start_date, end_date, is_active FROM increments WHERE id = ?1",
    )?;
    let mut rows = stmt.query([id])?;
    match rows.next()? {
        Some(row) => Ok(row_to_increment(row)?),
        None => Err(AppError::Config(format!("Increment {id} not found"))),
    }
}

fn row_to_increment(row: &rusqlite::Row) -> rusqlite::Result<Increment> {
    Ok(Increment {
        id: row.get(0)?,
        name: row.get(1)?,
        jql: row.get(2)?,
        start_date: parse_date_col(row.get::<_, String>(3)?),
        end_date: parse_date_col(row.get::<_, String>(4)?),
        is_active: row.get::<_, i64>(5)? != 0,
    })
}

pub fn upsert_increment(
    conn: &Connection,
    id: Option<i64>,
    name: &str,
    jql: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> AppResult<Increment> {
    let id = match id {
        Some(id) => {
            conn.execute(
                "UPDATE increments SET name=?1, jql=?2, start_date=?3, end_date=?4 WHERE id=?5",
                params![name, jql, start.to_string(), end.to_string(), id],
            )?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO increments(name, jql, start_date, end_date, is_active) VALUES (?1,?2,?3,?4,0)",
                params![name, jql, start.to_string(), end.to_string()],
            )?;
            conn.last_insert_rowid()
        }
    };
    // First increment becomes active automatically.
    let actives: i64 = conn.query_row("SELECT COUNT(*) FROM increments WHERE is_active = 1", [], |r| r.get(0))?;
    if actives == 0 {
        conn.execute("UPDATE increments SET is_active = 1 WHERE id = ?1", [id])?;
    }
    get_increment(conn, id)
}

pub fn delete_increment(conn: &Connection, id: i64) -> AppResult<()> {
    // Everything is keyed by increment_id, so removal is a clean per-increment
    // sweep that can't disturb another increment's cached rows.
    conn.execute("DELETE FROM issue_sprints WHERE increment_id = ?1", [id])?;
    conn.execute("DELETE FROM status_events WHERE increment_id = ?1", [id])?;
    conn.execute("DELETE FROM issues WHERE increment_id = ?1", [id])?;
    conn.execute("DELETE FROM epics WHERE increment_id = ?1", [id])?;
    conn.execute("DELETE FROM snapshots WHERE increment_id = ?1", [id])?;
    conn.execute("DELETE FROM sync_state WHERE increment_id = ?1", [id])?;
    conn.execute("DELETE FROM increments WHERE id = ?1", [id])?;
    Ok(())
}

pub fn set_active_increment(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("UPDATE increments SET is_active = 0", [])?;
    conn.execute("UPDATE increments SET is_active = 1 WHERE id = ?1", [id])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Sync write path
// ---------------------------------------------------------------------------

pub fn save_sync_result(
    conn: &mut Connection,
    increment_id: i64,
    epics: &[Epic],
    issues: &[Issue],
    sprints: &[Sprint],
) -> AppResult<()> {
    let tx = conn.transaction()?;

    // Epics that vanished from the JQL stay visible, badged "removed from plan".
    // Identity is (increment_id, key), so this only ever touches this increment.
    tx.execute(
        "UPDATE epics SET removed_from_plan = 1 WHERE increment_id = ?1",
        [increment_id],
    )?;
    for e in epics {
        tx.execute(
            "INSERT INTO epics(key, increment_id, name, owner, sp, start_date, end_date, status_category, carried_from, removed_from_plan)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0)
             ON CONFLICT(increment_id, key) DO UPDATE SET name=?3, owner=?4, sp=?5,
               start_date=?6, end_date=?7, status_category=?8, carried_from=?9, removed_from_plan=0",
            params![
                e.key,
                increment_id,
                e.name,
                e.owner,
                e.sp,
                e.start_date.map(|d| d.to_string()),
                e.end_date.map(|d| d.to_string()),
                e.status_category.as_str(),
                e.carried_from,
            ],
        )?;
    }

    // Replace this increment's issue data wholesale. Scoped strictly by
    // increment_id, so syncing one increment never deletes another's rows
    // (even when a Jira issue is shared across increments).
    tx.execute("DELETE FROM issue_sprints WHERE increment_id = ?1", [increment_id])?;
    tx.execute("DELETE FROM status_events WHERE increment_id = ?1", [increment_id])?;
    tx.execute("DELETE FROM issues WHERE increment_id = ?1", [increment_id])?;

    for i in issues {
        tx.execute(
            "INSERT INTO issues(key, increment_id, epic_key, summary, sp, effective_sp, sp_imputed, status, status_category,
                resolution, descoped, blocked, assignee, created, done_at, reopened, current_sprint_id, spill_count)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![
                i.key,
                increment_id,
                i.epic_key,
                i.summary,
                i.sp,
                i.effective_sp,
                i.sp_imputed as i64,
                i.status,
                i.status_category.as_str(),
                i.resolution,
                i.descoped as i64,
                i.blocked as i64,
                i.assignee,
                i.created.to_rfc3339(),
                i.done_at.map(|d| d.to_rfc3339()),
                i.reopened as i64,
                i.current_sprint_id,
                i.spill_count,
            ],
        )?;
        for l in &i.sprints {
            tx.execute(
                "INSERT OR REPLACE INTO issue_sprints(increment_id, issue_key, sprint_id, was_committed, added_mid_sprint, done_at_close)
                 VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    increment_id,
                    i.key,
                    l.sprint_id,
                    l.was_committed as i64,
                    l.added_mid_sprint as i64,
                    l.done_at_close.map(|b| b as i64),
                ],
            )?;
        }
        for ev in &i.status_events {
            tx.execute(
                "INSERT INTO status_events(increment_id, issue_key, at, from_category, to_category) VALUES (?1,?2,?3,?4,?5)",
                params![increment_id, i.key, ev.at.to_rfc3339(), ev.from.as_str(), ev.to.as_str()],
            )?;
        }
    }

    for s in sprints {
        tx.execute(
            "INSERT INTO sprints(id, name, state, start_date, end_date, board_id) VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET name=?2, state=?3, start_date=?4, end_date=?5, board_id=?6",
            params![
                s.id,
                s.name,
                s.state.as_str(),
                s.start_date.map(|d| d.to_rfc3339()),
                s.end_date.map(|d| d.to_rfc3339()),
                s.board_id,
            ],
        )?;
    }

    // Snapshot for the burn-up scope line + sync timestamp.
    let b = crate::domain::progress::breakdown(issues);
    let now = Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO snapshots(increment_id, taken_at, done_sp, scope_sp, in_progress_sp) VALUES (?1,?2,?3,?4,?5)",
        params![increment_id, now, b.done_sp, b.total_sp, b.in_progress_sp],
    )?;
    tx.execute(
        "INSERT INTO sync_state(increment_id, last_full_sync) VALUES (?1, ?2)
         ON CONFLICT(increment_id) DO UPDATE SET last_full_sync = ?2",
        params![increment_id, now],
    )?;

    tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Read path
// ---------------------------------------------------------------------------

pub struct IncrementBundle {
    pub increment: Increment,
    pub epics: Vec<Epic>,
    pub issues: Vec<Issue>,
    pub sprints: Vec<Sprint>,
    pub snapshots: Vec<Snapshot>,
    pub last_synced: Option<DateTime<Utc>>,
}

pub fn load_bundle(conn: &Connection, increment_id: i64) -> AppResult<IncrementBundle> {
    let increment = get_increment(conn, increment_id)?;

    let mut stmt = conn.prepare(
        "SELECT key, name, owner, sp, start_date, end_date, status_category, carried_from, removed_from_plan
         FROM epics WHERE increment_id = ?1",
    )?;
    let epics: Vec<Epic> = stmt
        .query_map([increment_id], |row| {
            Ok(Epic {
                key: row.get(0)?,
                name: row.get(1)?,
                owner: row.get(2)?,
                sp: row.get(3)?,
                start_date: row.get::<_, Option<String>>(4)?.map(parse_date_col),
                end_date: row.get::<_, Option<String>>(5)?.map(parse_date_col),
                status_category: StatusCategory::parse(&row.get::<_, String>(6)?),
                carried_from: row.get(7)?,
                removed_from_plan: row.get::<_, i64>(8)? != 0,
            })
        })?
        .collect::<Result<_, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT i.key, i.epic_key, i.summary, i.sp, i.effective_sp, i.sp_imputed, i.status, i.status_category,
                i.resolution, i.descoped, i.blocked, i.assignee, i.created, i.done_at, i.reopened,
                i.current_sprint_id, i.spill_count
         FROM issues i WHERE i.increment_id = ?1",
    )?;
    let mut issues: Vec<Issue> = stmt
        .query_map([increment_id], |row| {
            Ok(Issue {
                key: row.get(0)?,
                epic_key: row.get(1)?,
                summary: row.get(2)?,
                sp: row.get(3)?,
                effective_sp: row.get(4)?,
                sp_imputed: row.get::<_, i64>(5)? != 0,
                status: row.get(6)?,
                status_category: StatusCategory::parse(&row.get::<_, String>(7)?),
                resolution: row.get(8)?,
                descoped: row.get::<_, i64>(9)? != 0,
                blocked: row.get::<_, i64>(10)? != 0,
                assignee: row.get(11)?,
                created: parse_dt_col(row.get::<_, String>(12)?),
                done_at: row.get::<_, Option<String>>(13)?.map(parse_dt_col),
                reopened: row.get::<_, i64>(14)? != 0,
                current_sprint_id: row.get(15)?,
                spill_count: row.get::<_, i64>(16)? as u32,
                sprints: vec![],
                status_events: vec![],
            })
        })?
        .collect::<Result<_, _>>()?;

    // Attach sprint links and status events.
    {
        let mut link_stmt = conn.prepare(
            "SELECT sprint_id, was_committed, added_mid_sprint, done_at_close
             FROM issue_sprints WHERE increment_id = ?1 AND issue_key = ?2",
        )?;
        let mut ev_stmt = conn.prepare(
            "SELECT at, from_category, to_category
             FROM status_events WHERE increment_id = ?1 AND issue_key = ?2 ORDER BY at",
        )?;
        for issue in issues.iter_mut() {
            issue.sprints = link_stmt
                .query_map(params![increment_id, &issue.key], |row| {
                    Ok(IssueSprint {
                        sprint_id: row.get(0)?,
                        was_committed: row.get::<_, i64>(1)? != 0,
                        added_mid_sprint: row.get::<_, i64>(2)? != 0,
                        done_at_close: row.get::<_, Option<i64>>(3)?.map(|v| v != 0),
                    })
                })?
                .collect::<Result<_, _>>()?;
            issue.status_events = ev_stmt
                .query_map(params![increment_id, &issue.key], |row| {
                    Ok(StatusEvent {
                        at: parse_dt_col(row.get::<_, String>(0)?),
                        from: StatusCategory::parse(&row.get::<_, String>(1)?),
                        to: StatusCategory::parse(&row.get::<_, String>(2)?),
                    })
                })?
                .collect::<Result<_, _>>()?;
        }
    }

    let mut stmt = conn.prepare("SELECT id, name, state, start_date, end_date, board_id FROM sprints")?;
    let all_sprints: Vec<Sprint> = stmt
        .query_map([], |row| {
            Ok(Sprint {
                id: row.get(0)?,
                name: row.get(1)?,
                state: SprintState::parse(&row.get::<_, String>(2)?),
                start_date: row.get::<_, Option<String>>(3)?.map(parse_dt_col),
                end_date: row.get::<_, Option<String>>(4)?.map(parse_dt_col),
                board_id: row.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    // Only sprints referenced by this increment's issues.
    let referenced: std::collections::HashSet<i64> = issues
        .iter()
        .flat_map(|i| i.sprints.iter().map(|l| l.sprint_id))
        .collect();
    let sprints = all_sprints
        .into_iter()
        .filter(|s| referenced.contains(&s.id))
        .collect();

    let mut stmt = conn.prepare(
        "SELECT taken_at, done_sp, scope_sp, in_progress_sp FROM snapshots WHERE increment_id = ?1 ORDER BY taken_at",
    )?;
    let snapshots: Vec<Snapshot> = stmt
        .query_map([increment_id], |row| {
            Ok(Snapshot {
                taken_at: parse_dt_col(row.get::<_, String>(0)?),
                done_sp: row.get(1)?,
                scope_sp: row.get(2)?,
                in_progress_sp: row.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    let last_synced: Option<DateTime<Utc>> = {
        let mut stmt =
            conn.prepare("SELECT last_full_sync FROM sync_state WHERE increment_id = ?1")?;
        let mut rows = stmt.query([increment_id])?;
        rows.next()?
            .map(|row| row.get::<_, String>(0))
            .transpose()?
            .map(parse_dt_col)
    };

    Ok(IncrementBundle {
        increment,
        epics,
        issues,
        sprints,
        snapshots,
        last_synced,
    })
}

pub fn clear_all_data(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "DELETE FROM issue_sprints; DELETE FROM status_events; DELETE FROM issues;
         DELETE FROM epics; DELETE FROM sprints; DELETE FROM snapshots; DELETE FROM sync_state;",
    )?;
    Ok(())
}

fn parse_date_col(s: String) -> NaiveDate {
    NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap_or_default()
}

fn parse_dt_col(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::test_support::*;
    use crate::domain::model::StatusCategory::*;
    use crate::store::db::open_in_memory;

    #[test]
    fn round_trips_a_sync_result() {
        let mut conn = open_in_memory().unwrap();
        let inc = upsert_increment(
            &conn,
            None,
            "Increment 25",
            "fixVersion = \"Increment 25\"",
            date(2026, 1, 5),
            date(2026, 3, 27),
        )
        .unwrap();
        assert!(inc.is_active, "first increment should auto-activate");

        let epics = vec![epic("E-1", "Epic One")];
        let mut i = issue("A-1", "E-1", 5.0, InProgress);
        i.sprints = vec![IssueSprint {
            sprint_id: 1,
            was_committed: true,
            added_mid_sprint: false,
            done_at_close: Some(false),
        }];
        i.status_events = vec![StatusEvent {
            at: dt(2026, 1, 10),
            from: New,
            to: InProgress,
        }];
        i.spill_count = 1;
        let sprints = vec![sprint(1, "S1", SprintState::Closed, dt(2026, 1, 5), dt(2026, 1, 19))];

        save_sync_result(&mut conn, inc.id, &epics, &[i], &sprints).unwrap();

        let bundle = load_bundle(&conn, inc.id).unwrap();
        assert_eq!(bundle.epics.len(), 1);
        assert_eq!(bundle.issues.len(), 1);
        assert_eq!(bundle.sprints.len(), 1);
        assert_eq!(bundle.snapshots.len(), 1);
        assert!(bundle.last_synced.is_some());
        let loaded = &bundle.issues[0];
        assert_eq!(loaded.spill_count, 1);
        assert_eq!(loaded.sprints.len(), 1);
        assert_eq!(loaded.status_events.len(), 1);
        assert_eq!(loaded.sprints[0].done_at_close, Some(false));

        // Second sync without the epic keeps it, badged removed-from-plan.
        save_sync_result(&mut conn, inc.id, &[], &[], &[]).unwrap();
        let bundle = load_bundle(&conn, inc.id).unwrap();
        assert_eq!(bundle.epics.len(), 1);
        assert!(bundle.epics[0].removed_from_plan);
    }

    #[test]
    fn increments_sharing_an_epic_and_issue_stay_isolated() {
        // Defects A & B: a carried-forward epic (and its issue) legitimately
        // belongs to two increments at once. Syncing one must neither steal the
        // epic from the other (A) nor collide on the shared issue key (B).
        let mut conn = open_in_memory().unwrap();
        let a = upsert_increment(&conn, None, "Inc 24", "x", date(2025, 10, 1), date(2025, 12, 26)).unwrap();
        let b = upsert_increment(&conn, None, "Inc 25", "y", date(2026, 1, 5), date(2026, 3, 27)).unwrap();

        let epic = epic("SHARED-1", "Carried epic");
        let issue_in = |cat| {
            let mut i = issue("WORK-9", "SHARED-1", 5.0, cat);
            i.sprints = vec![IssueSprint {
                sprint_id: 1,
                was_committed: true,
                added_mid_sprint: false,
                done_at_close: None,
            }];
            i
        };
        let sprints = vec![sprint(1, "24:6", SprintState::Active, dt(2026, 1, 5), dt(2026, 1, 18))];

        // Sync the same epic/issue into both increments.
        save_sync_result(&mut conn, a.id, &[epic.clone()], &[issue_in(InProgress)], &sprints).unwrap();
        save_sync_result(&mut conn, b.id, &[epic.clone()], &[issue_in(Done)], &sprints).unwrap();

        // Each increment keeps its own copy (A: no theft).
        let ba = load_bundle(&conn, a.id).unwrap();
        let bb = load_bundle(&conn, b.id).unwrap();
        assert_eq!(ba.epics.len(), 1);
        assert_eq!(bb.epics.len(), 1);
        assert_eq!(ba.issues.len(), 1);
        assert_eq!(bb.issues.len(), 1);
        // ...with independent data (the issue's status differs per increment).
        assert_eq!(ba.issues[0].status_category, InProgress);
        assert_eq!(bb.issues[0].status_category, Done);
        assert_eq!(ba.issues[0].sprints.len(), 1);

        // Re-syncing one increment (B) must not crash on the shared key (B) and
        // must leave the other increment (A) untouched.
        save_sync_result(&mut conn, b.id, &[epic.clone()], &[issue_in(InProgress)], &sprints).unwrap();
        let ba2 = load_bundle(&conn, a.id).unwrap();
        assert_eq!(ba2.issues.len(), 1);
        assert_eq!(ba2.issues[0].status_category, InProgress); // A still has its own row
    }
}
