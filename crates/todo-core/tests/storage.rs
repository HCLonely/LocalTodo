use chrono::{NaiveDate, TimeZone, Utc};
use todo_core::*;
fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 8, 2, 0, 0).unwrap()
}
fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}
fn draft() -> TaskDraft {
    TaskDraft {
        title: "报告".into(),
        due_date: Some(d("2026-10-20")),
        reminder_days: vec![3, 1, 0],
        subtasks: vec![Subtask {
            title: "查资料".into(),
            completed: false,
        }],
        ..Default::default()
    }
}

#[test]
fn tasks_survive_reopen_and_delete_restore_preserves_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let mut store = Store::open(&path).unwrap();
    let t = store.save_task(None, draft(), "single", now()).unwrap();
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.get_task(&t.id).unwrap().draft.title, "报告");
    store.set_completed(&t.id, true, now()).unwrap();
    let done = store.get_task(&t.id).unwrap();
    assert!(done.completed && done.draft.subtasks[0].completed);
    store.set_completed(&t.id, false, now()).unwrap();
    assert!(store.get_task(&t.id).unwrap().draft.subtasks[0].completed);
    store.set_deleted(&t.id, true, now()).unwrap();
    assert!(store.get_task(&t.id).unwrap().deleted);
    store.set_deleted(&t.id, false, now()).unwrap();
    assert_eq!(store.get_task(&t.id).unwrap().draft.title, "报告");
}
#[test]
fn reminder_ticks_combine_missed_jobs_and_cancel_changed_tasks() {
    let mut store = Store::memory().unwrap();
    let t = store.save_task(None, draft(), "single", now()).unwrap();
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 2, 0, 0).unwrap();
    let mut batches = vec![];
    assert_eq!(
        store
            .tick(at, |b| {
                batches.push(b.clone());
                Ok(())
            })
            .unwrap(),
        1
    );
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].task_ids, vec![t.id.clone()]);
    assert_eq!(store.tick(at, |_| panic!("duplicate")).unwrap(), 0);
    let t2 = store.save_task(None, draft(), "single", now()).unwrap();
    store.set_completed(&t2.id, true, now()).unwrap();
    assert_eq!(store.tick(at, |_| panic!("completed task")).unwrap(), 0);
    let t3 = store.save_task(None, draft(), "single", now()).unwrap();
    let mut changed = t3.draft.clone();
    changed.due_date = Some(d("2026-11-20"));
    store
        .save_task(Some(&t3.id), changed, "single", now())
        .unwrap();
    assert_eq!(store.tick(at, |_| panic!("stale revision")).unwrap(), 0);
}
#[test]
fn notification_failures_keep_inbox_and_retry_without_losing_jobs() {
    let mut store = Store::memory().unwrap();
    store.save_task(None, draft(), "single", now()).unwrap();
    let at = Utc.with_ymd_and_hms(2026, 10, 17, 2, 0, 0).unwrap();
    store
        .tick(at, |_| Err(AppError::Validation("系统通知关闭".into())))
        .unwrap();
    assert_eq!(store.inbox().unwrap().len(), 1);
    assert!(!store.inbox().unwrap()[0].submitted);
    assert_eq!(store.tick(at, |_| panic!("retry too soon")).unwrap(), 0);
    assert_eq!(
        store
            .tick(at + chrono::Duration::minutes(1), |_| Ok(()))
            .unwrap(),
        1
    );
    assert_eq!(store.inbox().unwrap().len(), 1);
    assert!(store.inbox().unwrap()[0].submitted);
}
#[test]
fn monthly_instances_preserve_anchor_and_do_not_regenerate_deleted_occurrence() {
    let mut store = Store::memory().unwrap();
    let mut t = draft();
    t.due_date = Some(d("2027-01-31"));
    t.recurrence = Some(RecurrenceRule {
        kind: "monthly".into(),
        start: d("2027-01-31"),
        end: Some(d("2027-03-31")),
        weekday: None,
        month_day: Some(31),
    });
    let first = store.save_task(None, t, "single", now()).unwrap();
    while store
        .materialize_until(d("2027-03-31"), 100, now())
        .unwrap()
        .has_more
    {}
    let tasks = store.all_tasks().unwrap();
    let mut dates: Vec<_> = tasks
        .iter()
        .map(|t| t.draft.due_date.unwrap().to_string())
        .collect();
    dates.sort();
    assert_eq!(dates, vec!["2027-01-31", "2027-02-28", "2027-03-31"]);
    store.set_deleted(&first.id, true, now()).unwrap();
    assert_eq!(
        store
            .materialize_until(d("2027-03-31"), 100, now())
            .unwrap()
            .created,
        0
    );
}
#[test]
fn long_offline_generation_is_bounded_and_keeps_unfinished_instances() {
    let mut store = Store::memory().unwrap();
    let mut t = draft();
    t.due_date = Some(d("2024-01-01"));
    t.reminder_days = vec![];
    t.recurrence = Some(RecurrenceRule {
        kind: "daily".into(),
        start: d("2024-01-01"),
        end: Some(d("2025-12-31")),
        weekday: None,
        month_day: None,
    });
    store.save_task(None, t, "single", now()).unwrap();
    let mut passes = 0;
    loop {
        let p = store
            .materialize_until(d("2025-12-31"), 100, now())
            .unwrap();
        assert!(p.created <= 100);
        passes += 1;
        if !p.has_more {
            break;
        }
    }
    assert!(passes > 1);
    let ts = store.all_tasks().unwrap();
    assert_eq!(ts.len(), 731);
    assert!(ts.iter().all(|t| !t.completed));
}
#[test]
fn split_series_retains_completed_history_and_replaces_future() {
    let mut store = Store::memory().unwrap();
    let mut t = draft();
    t.due_date = Some(d("2026-10-08"));
    t.recurrence = Some(RecurrenceRule {
        kind: "daily".into(),
        start: d("2026-10-08"),
        end: Some(d("2026-10-11")),
        weekday: None,
        month_day: None,
    });
    let first = store.save_task(None, t, "single", now()).unwrap();
    store
        .materialize_until(d("2026-10-11"), 100, now())
        .unwrap();
    store.set_completed(&first.id, true, now()).unwrap();
    let target = store
        .all_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.draft.due_date == Some(d("2026-10-09")))
        .unwrap();
    let mut changed = target.draft.clone();
    changed.title = "新模板".into();
    store
        .save_task(Some(&target.id), changed, "following", now())
        .unwrap();
    store
        .materialize_until(d("2026-10-11"), 100, now())
        .unwrap();
    let ts = store.all_tasks().unwrap();
    assert!(ts
        .iter()
        .any(|t| t.id == first.id && t.completed && t.draft.title == "报告"));
    let active: Vec<_> = ts.iter().filter(|t| !t.deleted && !t.completed).collect();
    assert_eq!(active.len(), 3);
    assert!(active.iter().all(|t| t.draft.title == "新模板"));
}
#[test]
fn corrupted_import_does_not_mutate_and_valid_restore_does_not_replay_history() {
    let mut store = Store::memory().unwrap();
    let t = store.save_task(None, draft(), "single", now()).unwrap();
    let json = store.export_json().unwrap();
    assert!(store.restore_json("{broken", now()).is_err());
    assert_eq!(store.get_task(&t.id).unwrap().draft.title, "报告");
    let later = Utc.with_ymd_and_hms(2026, 10, 21, 2, 0, 0).unwrap();
    store.restore_json(&json, later).unwrap();
    assert_eq!(store.all_tasks().unwrap().len(), 1);
    assert_eq!(
        store
            .tick(later, |_| panic!("historical import notifications"))
            .unwrap(),
        0
    );
}
#[test]
fn read_only_database_reports_failed_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("read.db");
    drop(Store::open(&path).unwrap());
    let mut store = Store::open_read_only(&path).unwrap();
    assert!(store.save_task(None, draft(), "single", now()).is_err());
    assert_eq!(store.all_tasks().unwrap().len(), 0);
}
