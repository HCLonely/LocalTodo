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

#[test]
fn restoring_an_old_series_does_not_notify_for_backfilled_history() {
    let mut store = Store::memory().unwrap();
    let mut t = draft();
    t.due_date = Some(d("2026-01-01"));
    t.reminder_days = vec![0];
    t.recurrence = Some(RecurrenceRule {
        kind: "daily".into(),
        start: d("2026-01-01"),
        end: Some(d("2026-10-09")),
        weekday: None,
        month_day: None,
    });
    store
        .save_task(
            None,
            t,
            "single",
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        )
        .unwrap();
    let json = store.export_json().unwrap();
    store.restore_json(&json, now()).unwrap();
    while store
        .materialize_until(d("2026-10-09"), 100, now())
        .unwrap()
        .has_more
    {}
    assert_eq!(
        store
            .tick(now(), |_| panic!(
                "old backup must not replay historical reminders"
            ))
            .unwrap(),
        0
    );
    assert_eq!(
        store
            .tick(Utc.with_ymd_and_hms(2026, 10, 9, 2, 0, 0).unwrap(), |_| Ok(
                ()
            ))
            .unwrap(),
        1
    );
}

#[test]
fn editing_following_from_first_occurrence_keeps_archive_restorable() {
    let mut store = Store::memory().unwrap();
    let mut t = draft();
    t.due_date = Some(d("2026-10-08"));
    t.recurrence = Some(RecurrenceRule {
        kind: "daily".into(),
        start: d("2026-10-08"),
        end: Some(d("2026-10-10")),
        weekday: None,
        month_day: None,
    });
    let first = store.save_task(None, t, "single", now()).unwrap();
    let mut changed = first.draft.clone();
    changed.title = "调整后的任务".into();
    store
        .save_task(Some(&first.id), changed, "following", now())
        .unwrap();
    let archive = store.export_json().unwrap();
    assert!(store.restore_json(&archive, now()).is_ok());
}

fn daily(start: &str, end: &str) -> TaskDraft {
    let mut t = draft();
    t.due_date = Some(d(start));
    t.recurrence = Some(RecurrenceRule {
        kind: "daily".into(),
        start: d(start),
        end: Some(d(end)),
        weekday: None,
        month_day: None,
    });
    t
}
#[test]
fn recovery_catches_up_all_batches_before_submitting_one_summary() {
    let mut store = Store::memory().unwrap();
    let mut t = daily("2023-01-01", "2025-12-31");
    t.reminder_days = vec![0];
    store
        .save_task(
            None,
            t,
            "single",
            Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap(),
        )
        .unwrap();
    let mut submissions = 0;
    let mut count = 0;
    loop {
        let result = store
            .background_tick(now(), |batch| {
                submissions += 1;
                count = batch.task_ids.len();
                Ok(())
            })
            .unwrap();
        if !result.generation.has_more {
            break;
        }
    }
    assert_eq!(submissions, 1);
    assert_eq!(count, 1096);
    assert_eq!(
        store.tick(now(), |_| panic!("remaining backlog")).unwrap(),
        0
    );
}
#[test]
fn splitting_excludes_completed_future_occurrences_and_replaces_descendants() {
    let mut store = Store::memory().unwrap();
    let first = store
        .save_task(None, daily("2026-10-08", "2026-10-12"), "single", now())
        .unwrap();
    store
        .materialize_until(d("2026-10-12"), 100, now())
        .unwrap();
    let tasks = store.all_tasks().unwrap();
    let done = tasks
        .iter()
        .find(|t| t.draft.due_date == Some(d("2026-10-11")))
        .unwrap();
    store.set_completed(&done.id, true, now()).unwrap();
    let tenth = tasks
        .iter()
        .find(|t| t.draft.due_date == Some(d("2026-10-10")))
        .unwrap();
    let mut changed = tenth.draft.clone();
    changed.title = "第二段".into();
    store
        .save_task(Some(&tenth.id), changed, "following", now())
        .unwrap();
    store
        .materialize_until(d("2026-10-12"), 100, now())
        .unwrap();
    assert_eq!(
        store
            .all_tasks()
            .unwrap()
            .iter()
            .filter(|t| !t.deleted && t.draft.due_date == Some(d("2026-10-11")))
            .count(),
        1
    );
    let ninth = store
        .all_tasks()
        .unwrap()
        .into_iter()
        .find(|t| !t.deleted && t.draft.due_date == Some(d("2026-10-09")))
        .unwrap();
    let mut changed = ninth.draft.clone();
    changed.title = "重新安排".into();
    store
        .save_task(Some(&ninth.id), changed, "following", now())
        .unwrap();
    store
        .materialize_until(d("2026-10-12"), 100, now())
        .unwrap();
    let active: Vec<_> = store
        .all_tasks()
        .unwrap()
        .into_iter()
        .filter(|t| !t.deleted)
        .collect();
    assert_eq!(active.len(), 5);
    assert!(active.iter().any(|t| t.id == first.id));
    assert_eq!(
        active
            .iter()
            .filter(|t| !t.completed && t.draft.due_date.unwrap() >= d("2026-10-09"))
            .count(),
        3
    );
    let json = store.export_json().unwrap();
    store.restore_json(&json, now()).unwrap();
}
#[test]
fn timezone_change_does_not_rearm_a_submitted_offset() {
    let mut store = Store::memory().unwrap();
    let mut settings = store.settings().unwrap();
    settings.timezone = "Asia/Shanghai".into();
    store.update_settings(settings, now()).unwrap();
    let mut t = draft();
    t.due_date = Some(d("2026-10-08"));
    t.reminder_days = vec![0];
    store
        .save_task(
            None,
            t,
            "single",
            Utc.with_ymd_and_hms(2026, 10, 8, 0, 0, 0).unwrap(),
        )
        .unwrap();
    store
        .tick(Utc.with_ymd_and_hms(2026, 10, 8, 1, 0, 0).unwrap(), |_| {
            Ok(())
        })
        .unwrap();
    let mut settings = store.settings().unwrap();
    settings.timezone = "America/New_York".into();
    store.update_settings(settings, now()).unwrap();
    assert_eq!(
        store
            .tick(
                Utc.with_ymd_and_hms(2026, 10, 8, 13, 0, 0).unwrap(),
                |_| panic!("timezone must not replay submitted reminder")
            )
            .unwrap(),
        0
    );
}
#[test]
fn invalid_series_cursor_is_rejected_before_replacing_data() {
    let mut store = Store::memory().unwrap();
    let t = store
        .save_task(None, daily("2026-10-08", "2026-10-12"), "single", now())
        .unwrap();
    let mut archive: serde_json::Value =
        serde_json::from_str(&store.export_json().unwrap()).unwrap();
    archive["series"][0]["cursor"] = serde_json::json!("9998-12-31");
    assert!(store.restore_json(&archive.to_string(), now()).is_err());
    assert_eq!(store.get_task(&t.id).unwrap().draft.title, "报告");
}
