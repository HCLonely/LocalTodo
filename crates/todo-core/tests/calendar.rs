use chrono::{NaiveDate, TimeZone, Utc};
use chrono_tz::{America::New_York, Asia::Shanghai};
use todo_core::*;

fn date(s: &str) -> NaiveDate {
    s.parse().unwrap()
}
fn ctx() -> CalendarContext {
    CalendarContext {
        now: Utc.with_ymd_and_hms(2026, 10, 8, 2, 0, 0).unwrap(),
        zone: Shanghai,
    }
}

#[test]
fn today_keeps_overdue_tasks_after_completion_and_excludes_deleted_tasks() {
    let mut task = Task::new(
        TaskDraft {
            title: "逾期报告".into(),
            due_date: Some(date("2026-10-07")),
            ..Default::default()
        },
        ctx().now,
    )
    .unwrap();
    assert!(matches_view(&task, "today", None, &ctx()).unwrap());
    assert!(is_overdue(&task, &ctx()).unwrap());
    task.completed = true;
    // Shanghai's October 8 starts on October 7 in UTC.
    task.completed_at = Some(Utc.with_ymd_and_hms(2026, 10, 7, 17, 0, 0).unwrap());
    assert!(matches_view(&task, "today", None, &ctx()).unwrap());
    assert!(!is_overdue(&task, &ctx()).unwrap());
    assert!(!matches_view(&task, "all", None, &ctx()).unwrap());
    task.completed_at = Some(Utc.with_ymd_and_hms(2026, 10, 7, 15, 0, 0).unwrap());
    assert!(!matches_view(&task, "today", None, &ctx()).unwrap());
    task.draft.planned_date = Some(date("2026-10-08"));
    assert!(matches_view(&task, "today", None, &ctx()).unwrap());
    task.deleted = true;
    assert!(!matches_view(&task, "today", None, &ctx()).unwrap());
}

#[test]
fn date_views_share_a_task_without_changing_its_deadline() {
    let mut task = Task::new(
        TaskDraft {
            title: "提交报告".into(),
            due_date: Some(date("2026-10-12")),
            ..Default::default()
        },
        ctx().now,
    )
    .unwrap();
    assert!(!matches_view(&task, "week", None, &ctx()).unwrap());
    assert!(matches_view(&task, "month", None, &ctx()).unwrap());
    task.draft.planned_date = Some(date("2026-10-08"));
    assert!(matches_view(&task, "today", None, &ctx()).unwrap());
    assert_eq!(task.draft.due_date, Some(date("2026-10-12")));
}
#[test]
fn date_only_deadline_expires_at_next_midnight() {
    let task = Task::new(
        TaskDraft {
            title: "今天".into(),
            due_date: Some(date("2026-10-08")),
            ..Default::default()
        },
        ctx().now,
    )
    .unwrap();
    assert_eq!(
        due_boundary(&task.draft, Shanghai)
            .unwrap()
            .unwrap()
            .to_rfc3339(),
        "2026-10-08T16:00:00+00:00"
    );
    assert!(!is_overdue(&task, &ctx()).unwrap());
    let at_boundary = CalendarContext {
        now: Utc.with_ymd_and_hms(2026, 10, 8, 16, 0, 0).unwrap(),
        ..ctx()
    };
    assert!(is_overdue(&task, &at_boundary).unwrap());
}
#[test]
fn year_crossing_week_uses_monday_through_sunday() {
    let c = CalendarContext {
        now: Utc.with_ymd_and_hms(2026, 12, 31, 2, 0, 0).unwrap(),
        ..ctx()
    };
    let task = Task::new(
        TaskDraft {
            title: "跨年".into(),
            due_date: Some(date("2027-01-03")),
            ..Default::default()
        },
        c.now,
    )
    .unwrap();
    assert!(matches_view(&task, "week", None, &c).unwrap());
    assert!(!matches_view(&task, "month", None, &c).unwrap());
}
#[test]
fn validation_rejects_blank_title_and_impossible_reminders() {
    assert!(Task::new(TaskDraft::default(), ctx().now).is_err());
    let mut d = TaskDraft {
        title: "任务".into(),
        reminder_days: vec![1],
        ..Default::default()
    };
    assert!(d.validate().is_err());
    d.due_date = Some(date("2026-10-20"));
    d.reminder_days = vec![366];
    assert!(d.validate().is_err());
    d.reminder_days = vec![1, 1, 0];
    d.validate().unwrap();
    assert_eq!(d.reminder_days, vec![0, 1]);
    d.due_time = Some("08:00:00".parse().unwrap());
    assert!(d.validate().is_err());
    assert!("2027-02-29".parse::<NaiveDate>().is_err());
}
#[test]
fn dst_gap_moves_to_next_valid_minute_and_fold_uses_earlier_instant() {
    let gap = local_instant(date("2026-03-08"), "02:30:00".parse().unwrap(), New_York).unwrap();
    assert_eq!(gap.to_rfc3339(), "2026-03-08T07:00:00+00:00");
    let fold = local_instant(date("2026-11-01"), "01:30:00".parse().unwrap(), New_York).unwrap();
    assert_eq!(fold.to_rfc3339(), "2026-11-01T05:30:00+00:00");
}
