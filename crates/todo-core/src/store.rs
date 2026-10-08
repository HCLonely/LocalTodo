use crate::*;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    time::Duration as StdDuration,
};
use uuid::Uuid;

pub struct Store {
    conn: Connection,
}
#[derive(Serialize, Deserialize)]
struct Archive {
    schema_version: u32,
    tasks: Vec<Task>,
    series: Vec<Series>,
    settings: Settings,
}

fn load_payloads<T: serde::de::DeserializeOwned>(
    conn: &Connection,
    table: &str,
) -> AppResult<Vec<T>> {
    // Table names are internal constants, never user input.
    let mut stmt = conn.prepare(&format!("SELECT payload FROM {table}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.map(|s| Ok(serde_json::from_str(&s?)?)).collect()
}
fn task_from(conn: &Connection, id: &str) -> AppResult<Task> {
    let json: Option<String> = conn
        .query_row("SELECT payload FROM tasks WHERE id=?", [id], |r| r.get(0))
        .optional()?;
    Ok(serde_json::from_str(&json.ok_or(AppError::NotFound)?)?)
}
fn write_task(conn: &Connection, t: &Task) -> AppResult<()> {
    conn.execute("INSERT INTO tasks(id,payload,due_date,planned_date,completed,deleted,series_id,occurrence_date) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,due_date=excluded.due_date,planned_date=excluded.planned_date,completed=excluded.completed,deleted=excluded.deleted,series_id=excluded.series_id,occurrence_date=excluded.occurrence_date",params![t.id,serde_json::to_string(t)?,t.draft.due_date.map(|d|d.to_string()),t.draft.planned_date.map(|d|d.to_string()),t.completed,t.deleted,t.series_id,t.occurrence_date.map(|d|d.to_string())])?;
    Ok(())
}
fn write_series(conn: &Connection, s: &Series) -> AppResult<()> {
    conn.execute("INSERT INTO series(id,payload) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![s.id,serde_json::to_string(s)?])?;
    Ok(())
}
fn write_inbox(conn: &Connection, b: &NotificationBatch) -> AppResult<()> {
    conn.execute("INSERT INTO inbox(id,payload) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![b.id,serde_json::to_string(b)?])?;
    Ok(())
}
fn settings_from(conn: &Connection) -> AppResult<Settings> {
    let json: String =
        conn.query_row("SELECT payload FROM settings WHERE id=1", [], |r| r.get(0))?;
    Ok(serde_json::from_str(&json)?)
}
fn rebuild_jobs(
    conn: &Connection,
    t: &Task,
    zone: chrono_tz::Tz,
    now: DateTime<Utc>,
    include_past: bool,
) -> AppResult<()> {
    conn.execute(
        "UPDATE jobs SET status='cancelled' WHERE task_id=? AND status='pending'",
        [&t.id],
    )?;
    if t.completed || t.deleted {
        return Ok(());
    }
    let mut missed = false;
    for (offset, at) in reminder_times(&t.draft, zone)? {
        let past = at < now;
        missed |= past;
        conn.execute("INSERT OR IGNORE INTO jobs(id,task_id,revision,offset_days,trigger_at,status) VALUES(?,?,?,?,?,?)",params![Uuid::new_v4().to_string(),t.id,t.revision,offset,at.timestamp(),if past&&!include_past{"cancelled"}else{"pending"}])?;
    }
    if missed && !include_past {
        write_inbox(
            conn,
            &NotificationBatch {
                id: Uuid::new_v4().to_string(),
                title: "提醒时间已过".into(),
                body: format!("{} 的部分提醒时间已过，请检查截止日期。", t.draft.title),
                task_ids: vec![t.id.clone()],
                created_at: now,
                submitted: false,
                error: None,
                read: false,
            },
        )?;
    }
    Ok(())
}
fn instance(series: &Series, date: NaiveDate, now: DateTime<Utc>) -> AppResult<Task> {
    let mut draft = series.template.clone();
    if let (Some(planned), Some(anchor)) = (draft.planned_date, draft.due_date) {
        draft.planned_date = planned.checked_add_signed(date - anchor);
    }
    draft.due_date = Some(date);
    draft.recurrence = Some(series.rule.clone());
    for s in &mut draft.subtasks {
        s.completed = false;
    }
    let mut t = Task::new(draft, now)?;
    t.series_id = Some(series.id.clone());
    t.occurrence_date = Some(date);
    Ok(t)
}

impl Store {
    pub fn memory() -> AppResult<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 1 {
            return Err(invalid("数据库来自更新版本，请升级程序后打开"));
        }
        if version == 0 && path.metadata()?.len() > 0 {
            let backup = path.with_extension(format!(
                "pre-migration-{}.db",
                Utc::now().format("%Y%m%d%H%M%S%f")
            ));
            let mut dest = Connection::open(backup)?;
            rusqlite::backup::Backup::new(&conn, &mut dest)?.run_to_completion(
                100,
                StdDuration::from_millis(10),
                None,
            )?;
        }
        Self::initialize(conn)
    }
    pub fn open_read_only(path: &Path) -> AppResult<Self> {
        Ok(Self {
            conn: Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?,
        })
    }
    fn initialize(mut conn: Connection) -> AppResult<Self> {
        conn.busy_timeout(StdDuration::from_secs(5))?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("schema.sql"))?;
        tx.execute(
            "INSERT OR IGNORE INTO settings(id,payload) VALUES(1,?)",
            [serde_json::to_string(&Settings::default())?],
        )?;
        tx.commit()?;
        Ok(Self { conn })
    }
    pub fn settings(&self) -> AppResult<Settings> {
        settings_from(&self.conn)
    }
    pub fn all_tasks(&self) -> AppResult<Vec<Task>> {
        load_payloads(&self.conn, "tasks")
    }
    pub fn get_task(&self, id: &str) -> AppResult<Task> {
        task_from(&self.conn, id)
    }
    pub fn inbox(&self) -> AppResult<Vec<NotificationBatch>> {
        let mut items: Vec<NotificationBatch> = load_payloads(&self.conn, "inbox")?;
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items.truncate(100);
        Ok(items)
    }
    pub fn mark_inbox_read(&mut self) -> AppResult<()> {
        let all: Vec<NotificationBatch> = load_payloads(&self.conn, "inbox")?;
        let tx = self.conn.transaction()?;
        for mut b in all {
            if !b.read {
                b.read = true;
                write_inbox(&tx, &b)?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn save_task(
        &mut self,
        id: Option<&str>,
        mut draft: TaskDraft,
        scope: &str,
        now: DateTime<Utc>,
    ) -> AppResult<Task> {
        draft.validate()?;
        if !["single", "following"].contains(&scope) {
            return Err(invalid("修改范围无效"));
        }
        let zone = self.settings()?.zone()?;
        let old = id.map(|v| self.get_task(v)).transpose()?;
        if old.as_ref().is_some_and(|t| t.deleted) {
            return Err(invalid("请先从回收站恢复任务"));
        }
        let tx = self.conn.transaction()?;
        let task = if let Some(mut old) = old {
            if scope == "following" && old.series_id.is_some() {
                if old.completed {
                    return Err(invalid("已完成任务只能编辑本次，不能改变历史系列"));
                }
                let sid = old.series_id.as_ref().unwrap();
                let json: String =
                    tx.query_row("SELECT payload FROM series WHERE id=?", [sid], |r| r.get(0))?;
                let mut series: Series = serde_json::from_str(&json)?;
                let cutoff = old
                    .occurrence_date
                    .ok_or_else(|| invalid("重复实例缺少日期"))?;
                series.rule.end = cutoff.pred_opt();
                if cutoff <= series.rule.start {
                    series.active = false;
                }
                write_series(&tx, &series)?;
                let tasks: Vec<Task> = load_payloads(&tx, "tasks")?;
                for mut t in tasks {
                    if t.series_id.as_ref() == Some(sid)
                        && t.occurrence_date.is_some_and(|d| d >= cutoff)
                        && !t.completed
                        && !t.deleted
                    {
                        t.deleted = true;
                        t.updated_at = now;
                        t.revision += 1;
                        write_task(&tx, &t)?;
                        rebuild_jobs(&tx, &t, zone, now, false)?;
                    }
                }
                if let Some(rule) = &mut draft.recurrence {
                    rule.start = cutoff;
                    rule.validate()?;
                }
                Self::create_in_transaction(&tx, draft, now, zone)?
            } else {
                if old.series_id.is_some() && draft.recurrence != old.draft.recurrence {
                    return Err(invalid("修改重复规则请选择“本次及以后”"));
                }
                if old.series_id.is_none() && draft.recurrence.is_some() {
                    old.deleted = true;
                    old.updated_at = now;
                    old.revision += 1;
                    write_task(&tx, &old)?;
                    rebuild_jobs(&tx, &old, zone, now, false)?;
                    Self::create_in_transaction(&tx, draft, now, zone)?
                } else {
                    old.draft = draft;
                    old.updated_at = now;
                    old.revision += 1;
                    write_task(&tx, &old)?;
                    rebuild_jobs(&tx, &old, zone, now, false)?;
                    old
                }
            }
        } else {
            Self::create_in_transaction(&tx, draft, now, zone)?
        };
        tx.commit()?;
        Ok(task)
    }
    fn create_in_transaction(
        tx: &Transaction<'_>,
        draft: TaskDraft,
        now: DateTime<Utc>,
        zone: chrono_tz::Tz,
    ) -> AppResult<Task> {
        let task = if let Some(rule) = draft.recurrence.clone() {
            let first = occurrence_on_or_after(&rule, rule.start)?
                .ok_or_else(|| invalid("结束日期范围内没有重复任务"))?;
            let series = Series {
                id: Uuid::new_v4().to_string(),
                template: draft,
                rule,
                cursor: Some(first),
                active: true,
            };
            let t = instance(&series, first, now)?;
            write_series(tx, &series)?;
            t
        } else {
            Task::new(draft, now)?
        };
        write_task(tx, &task)?;
        rebuild_jobs(tx, &task, zone, now, false)?;
        Ok(task)
    }
    pub fn set_completed(&mut self, id: &str, value: bool, now: DateTime<Utc>) -> AppResult<Task> {
        let mut t = self.get_task(id)?;
        if t.deleted {
            return Err(invalid("请先恢复任务"));
        }
        t.completed = value;
        t.completed_at = value.then_some(now);
        if value {
            for sub in &mut t.draft.subtasks {
                sub.completed = true;
            }
        }
        t.updated_at = now;
        t.revision += 1;
        let zone = self.settings()?.zone()?;
        let tx = self.conn.transaction()?;
        write_task(&tx, &t)?;
        rebuild_jobs(&tx, &t, zone, now, false)?;
        tx.commit()?;
        Ok(t)
    }
    pub fn set_deleted(&mut self, id: &str, value: bool, now: DateTime<Utc>) -> AppResult<()> {
        let mut t = self.get_task(id)?;
        t.deleted = value;
        t.updated_at = now;
        t.revision += 1;
        let zone = self.settings()?.zone()?;
        let tx = self.conn.transaction()?;
        write_task(&tx, &t)?;
        rebuild_jobs(&tx, &t, zone, now, false)?;
        tx.commit()?;
        Ok(())
    }
    pub fn materialize_until(
        &mut self,
        horizon: NaiveDate,
        limit: usize,
        now: DateTime<Utc>,
    ) -> AppResult<GenerationProgress> {
        valid_date(horizon)?;
        let limit = limit.clamp(1, 100);
        let zone = self.settings()?.zone()?;
        let series: Vec<Series> = load_payloads(&self.conn, "series")?;
        let mut created = 0;
        let mut has_more = false;
        let tx = self.conn.transaction()?;
        for mut s in series {
            if !s.active {
                continue;
            }
            let mut cursor = s
                .cursor
                .map(|d| d.succ_opt().ok_or_else(|| invalid("日期溢出")))
                .transpose()?
                .unwrap_or(s.rule.start);
            while let Some(date) = occurrence_on_or_after(&s.rule, cursor)? {
                if date > horizon {
                    break;
                }
                if created >= limit {
                    has_more = true;
                    break;
                }
                let exists: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM tasks WHERE series_id=? AND occurrence_date=?)",
                    params![s.id, date.to_string()],
                    |r| r.get(0),
                )?;
                if !exists {
                    let t = instance(&s, date, now)?;
                    write_task(&tx, &t)?;
                    rebuild_jobs(&tx, &t, zone, now, true)?;
                    created += 1;
                }
                s.cursor = Some(date);
                cursor = date.succ_opt().ok_or_else(|| invalid("日期溢出"))?;
            }
            write_series(&tx, &s)?;
        }
        tx.commit()?;
        Ok(GenerationProgress { created, has_more })
    }
    pub fn generation_horizon(
        &self,
        now: DateTime<Utc>,
        selected: Option<NaiveDate>,
    ) -> AppResult<NaiveDate> {
        let today = now.with_timezone(&self.settings()?.zone()?).date_naive();
        let month_next = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
        }
        .ok_or_else(|| invalid("日期溢出"))?;
        let series: Vec<Series> = load_payloads(&self.conn, "series")?;
        let max_offset = series
            .iter()
            .filter(|s| s.active)
            .flat_map(|s| s.template.reminder_days.iter().copied())
            .max()
            .unwrap_or(0);
        Ok(month_next
            .pred_opt()
            .unwrap()
            .max(today + Duration::days(i64::from(max_offset) + 7))
            .max(selected.unwrap_or(today)))
    }
    pub fn snapshot(&mut self, query: &Query, now: DateTime<Utc>) -> AppResult<Snapshot> {
        if let Some(date) = query.selected_date {
            valid_date(date)?;
        }
        let horizon = self.generation_horizon(now, query.selected_date)?;
        let progress = self.materialize_until(horizon, 100, now)?;
        let settings = self.settings()?;
        let ctx = CalendarContext {
            now,
            zone: settings.zone()?,
        };
        let today = now.with_timezone(&ctx.zone).date_naive();
        let tasks = self.all_tasks()?;
        let views = [
            "today",
            "week",
            "month",
            "date",
            "inbox",
            "overdue",
            "completed",
            "trash",
            "all",
        ];
        let mut counts: BTreeMap<String, usize> =
            views.iter().map(|v| (v.to_string(), 0)).collect();
        let mut overdue_ids = vec![];
        let mut filtered = vec![];
        let search = query.search.trim().to_lowercase();
        for t in tasks {
            for v in views {
                if matches_view(&t, v, query.selected_date, &ctx)? {
                    *counts.get_mut(v).unwrap() += 1;
                }
            }
            if is_overdue(&t, &ctx)? {
                overdue_ids.push(t.id.clone());
            }
            if matches_view(&t, &query.view, query.selected_date, &ctx)?
                && query.priority.is_none_or(|p| t.draft.priority == p)
                && (search.is_empty()
                    || format!(
                        "{} {} {}",
                        t.draft.title,
                        t.draft.note,
                        t.draft
                            .subtasks
                            .iter()
                            .map(|s| s.title.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                    .to_lowercase()
                    .contains(&search))
            {
                filtered.push(t);
            }
        }
        filtered.sort_by(|a, b| {
            a.draft
                .due_date
                .unwrap_or(NaiveDate::MAX)
                .cmp(&b.draft.due_date.unwrap_or(NaiveDate::MAX))
                .then(
                    a.draft
                        .due_time
                        .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap())
                        .cmp(
                            &b.draft
                                .due_time
                                .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
                        ),
                )
                .then(b.draft.priority.cmp(&a.draft.priority))
                .then(a.created_at.cmp(&b.created_at))
        });
        let total = filtered.len();
        let tasks = filtered
            .into_iter()
            .skip(query.offset)
            .take(query.limit.clamp(1, 100))
            .collect();
        Ok(Snapshot {
            tasks,
            total,
            counts,
            overdue_ids,
            inbox: self.inbox()?,
            settings,
            today,
            generation_pending: progress.has_more,
        })
    }
    pub fn tick(
        &mut self,
        now: DateTime<Utc>,
        mut notify: impl FnMut(&NotificationBatch) -> AppResult<()>,
    ) -> AppResult<usize> {
        let mut stmt=self.conn.prepare("SELECT id,task_id,revision,attempts,batch_id FROM jobs WHERE status='pending' AND trigger_at<=? AND (retry_at IS NULL OR retry_at<=?) ORDER BY trigger_at LIMIT 1000")?;
        let rows: Vec<(String, String, u32, u32, Option<String>)> = stmt
            .query_map(params![now.timestamp(), now.timestamp()], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .collect::<Result<_, _>>()?;
        drop(stmt);
        if rows.is_empty() {
            return Ok(0);
        }
        let mut valid = vec![];
        let mut tasks = BTreeMap::new();
        for row in rows {
            let task = self.get_task(&row.1)?;
            if task.deleted || task.completed || task.revision != row.2 {
                self.conn
                    .execute("UPDATE jobs SET status='cancelled' WHERE id=?", [&row.0])?;
            } else {
                tasks.insert(task.id.clone(), task);
                valid.push(row);
            }
        }
        if valid.is_empty() {
            return Ok(0);
        }
        let batch_id = valid
            .iter()
            .find_map(|r| r.4.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let names = tasks
            .values()
            .take(3)
            .map(|t| t.draft.title.as_str())
            .collect::<Vec<_>>()
            .join("、");
        let mut batch = NotificationBatch {
            id: batch_id.clone(),
            title: format!("你有 {} 项任务需要留意", tasks.len()),
            body: format!(
                "{names}{}",
                if tasks.len() > 3 {
                    " 等任务已到提醒时间"
                } else {
                    " 已到提醒时间"
                }
            ),
            task_ids: tasks.keys().cloned().collect(),
            created_at: now,
            submitted: false,
            error: None,
            read: false,
        };
        {
            let tx = self.conn.transaction()?;
            write_inbox(&tx, &batch)?;
            for row in &valid {
                tx.execute(
                    "UPDATE jobs SET batch_id=? WHERE id=?",
                    params![batch_id, row.0],
                )?;
            }
            tx.commit()?;
        }
        let result = notify(&batch);
        let success = result.is_ok();
        batch.submitted = success;
        batch.error = result.err().map(|e| e.to_string());
        let tx = self.conn.transaction()?;
        write_inbox(&tx, &batch)?;
        for row in valid {
            if success {
                tx.execute("UPDATE jobs SET status='submitted' WHERE id=?", [row.0])?;
            } else {
                let attempt = row.3 + 1;
                let status = if attempt > 3 { "failed" } else { "pending" };
                let delay = match attempt {
                    1 => 60,
                    2 => 300,
                    _ => 900,
                };
                tx.execute(
                    "UPDATE jobs SET attempts=?,status=?,retry_at=? WHERE id=?",
                    params![attempt, status, now.timestamp() + delay, row.0],
                )?;
            }
        }
        tx.commit()?;
        Ok(usize::from(success))
    }
    pub fn next_wake_seconds(&self, now: DateTime<Utc>) -> AppResult<u64> {
        let at:Option<i64>=self.conn.query_row("SELECT MIN(MAX(trigger_at,COALESCE(retry_at,trigger_at))) FROM jobs WHERE status='pending'",[],|r|r.get(0))?;
        Ok(at
            .map(|a| (a - now.timestamp()).clamp(1, 30) as u64)
            .unwrap_or(30))
    }
    pub fn update_settings(&mut self, settings: Settings, now: DateTime<Utc>) -> AppResult<()> {
        settings.validate()?;
        let old = self.settings()?;
        let tasks = if old.timezone != settings.timezone {
            self.all_tasks()?
        } else {
            vec![]
        };
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE settings SET payload=? WHERE id=1",
            [serde_json::to_string(&settings)?],
        )?;
        for mut t in tasks {
            t.revision += 1;
            write_task(&tx, &t)?;
            rebuild_jobs(&tx, &t, settings.zone()?, now, false)?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn export_json(&self) -> AppResult<String> {
        Ok(serde_json::to_string_pretty(&Archive {
            schema_version: 1,
            tasks: self.all_tasks()?,
            series: load_payloads(&self.conn, "series")?,
            settings: self.settings()?,
        })?)
    }
    pub fn restore_json(&mut self, json: &str, now: DateTime<Utc>) -> AppResult<usize> {
        if json.len() > 100 * 1024 * 1024 {
            return Err(invalid("备份文件超过100MB"));
        }
        let mut archive: Archive = serde_json::from_str(json)?;
        if archive.schema_version != 1 {
            return Err(invalid("不支持该备份版本"));
        }
        archive.settings.validate()?;
        let mut ids = HashSet::new();
        let mut series_ids = HashSet::new();
        let mut occurrences = HashSet::new();
        for s in &mut archive.series {
            Uuid::parse_str(&s.id).map_err(|_| invalid("备份系列ID无效"))?;
            s.rule.validate()?;
            s.template.validate()?;
            if !series_ids.insert(s.id.clone()) {
                return Err(invalid("备份含重复系列ID"));
            }
        }
        for t in &mut archive.tasks {
            Uuid::parse_str(&t.id).map_err(|_| invalid("备份任务ID无效"))?;
            t.draft.validate()?;
            if !ids.insert(t.id.clone()) {
                return Err(invalid("备份含重复任务ID"));
            }
            if let Some(sid) = &t.series_id {
                if !series_ids.contains(sid)
                    || t.occurrence_date.is_none()
                    || !occurrences.insert((sid.clone(), t.occurrence_date))
                {
                    return Err(invalid("备份重复实例引用无效"));
                }
            }
            t.revision = t
                .revision
                .checked_add(1)
                .ok_or_else(|| invalid("任务版本溢出"))?;
        }
        // Keep a recovery snapshot before replacement. In-memory stores use an in-memory snapshot.
        let recovery_path = self.conn.path().filter(|p| !p.is_empty()).map(|p| {
            Path::new(p).with_extension(format!(
                "before-restore-{}.db",
                now.format("%Y%m%d%H%M%S%f")
            ))
        });
        if let Some(path) = recovery_path {
            self.backup_sqlite(&path)?;
        }
        let count = archive.tasks.len();
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "DELETE FROM jobs; DELETE FROM inbox; DELETE FROM tasks; DELETE FROM series;",
        )?;
        tx.execute(
            "UPDATE settings SET payload=? WHERE id=1",
            [serde_json::to_string(&archive.settings)?],
        )?;
        for s in &archive.series {
            write_series(&tx, s)?;
        }
        for t in &archive.tasks {
            write_task(&tx, t)?;
            rebuild_jobs(&tx, t, archive.settings.zone()?, now, false)?;
        }
        tx.commit()?;
        Ok(count)
    }
    pub fn backup_sqlite(&self, path: &Path) -> AppResult<()> {
        if self.conn.path().is_some_and(|p| Path::new(p) == path) {
            return Err(invalid("备份路径不能是当前数据库"));
        }
        let mut dest = Connection::open(path)?;
        rusqlite::backup::Backup::new(&self.conn, &mut dest)?.run_to_completion(
            100,
            StdDuration::from_millis(10),
            None,
        )?;
        Ok(())
    }
}
