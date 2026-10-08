use crate::{invalid, AppResult, RecurrenceRule, Task, TaskDraft};
use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

pub struct CalendarContext {
    pub now: DateTime<Utc>,
    pub zone: Tz,
}
pub fn local_instant(date: NaiveDate, time: NaiveTime, zone: Tz) -> AppResult<DateTime<Utc>> {
    let mut local = date.and_time(time);
    for _ in 0..=2880 {
        match zone.from_local_datetime(&local) {
            LocalResult::Single(dt) => return Ok(dt.with_timezone(&Utc)),
            LocalResult::Ambiguous(a, b) => return Ok(a.min(b).with_timezone(&Utc)),
            LocalResult::None => {
                local = local
                    .checked_add_signed(Duration::minutes(1))
                    .ok_or_else(|| invalid("日期溢出"))?;
            }
        }
    }
    Err(invalid("无法解析该日期的本地时刻"))
}
pub fn due_boundary(draft: &TaskDraft, zone: Tz) -> AppResult<Option<DateTime<Utc>>> {
    let Some(date) = draft.due_date else {
        return Ok(None);
    };
    let (date, time) = match draft.due_time {
        Some(t) => (date, t),
        None => (
            date.succ_opt().ok_or_else(|| invalid("日期溢出"))?,
            NaiveTime::MIN,
        ),
    };
    Ok(Some(local_instant(date, time, zone)?))
}
pub fn is_overdue(task: &Task, ctx: &CalendarContext) -> AppResult<bool> {
    Ok(!task.completed
        && !task.deleted
        && due_boundary(&task.draft, ctx.zone)?.is_some_and(|at| ctx.now >= at))
}
pub fn matches_view(
    task: &Task,
    view: &str,
    selected: Option<NaiveDate>,
    ctx: &CalendarContext,
) -> AppResult<bool> {
    if view == "trash" {
        return Ok(task.deleted);
    }
    if task.deleted {
        return Ok(false);
    }
    if view == "completed" {
        return Ok(task.completed);
    }
    let today = ctx.now.with_timezone(&ctx.zone).date_naive();
    let due = task.draft.due_date;
    if view == "today" {
        return Ok(due == Some(today)
            || task.draft.planned_date == Some(today)
            || is_overdue(task, ctx)?
            || (task.completed
                && task
                    .completed_at
                    .is_some_and(|at| at.with_timezone(&ctx.zone).date_naive() == today)));
    }
    if task.completed {
        return Ok(false);
    }
    Ok(match view {
        "all" => true,
        "inbox" => due.is_none() && task.draft.planned_date.is_none(),
        "date" => due == selected && selected.is_some(),
        "overdue" => is_overdue(task, ctx)?,
        "week" => {
            let start = today - Duration::days(today.weekday().num_days_from_monday().into());
            due.is_some_and(|d| d >= start && d < start + Duration::days(7))
                && !is_overdue(task, ctx)?
        }
        "month" => {
            due.is_some_and(|d| d.year() == today.year() && d.month() == today.month())
                && !is_overdue(task, ctx)?
        }
        _ => return Err(invalid("任务视图无效")),
    })
}
pub fn occurrence_on_or_after(
    rule: &RecurrenceRule,
    date: NaiveDate,
) -> AppResult<Option<NaiveDate>> {
    rule.validate()?;
    let candidate = date.max(rule.start);
    let result = match rule.kind.as_str() {
        "daily" => candidate,
        "weekly" => {
            candidate
                + Duration::days(
                    ((rule.weekday.unwrap() + 7 - candidate.weekday().num_days_from_monday()) % 7)
                        .into(),
                )
        }
        "monthly" => {
            let mut y = candidate.year();
            let mut m = candidate.month();
            loop {
                let start =
                    NaiveDate::from_ymd_opt(y, m, 1).ok_or_else(|| invalid("重复日期溢出"))?;
                let next = if m == 12 {
                    NaiveDate::from_ymd_opt(y + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(y, m + 1, 1)
                }
                .ok_or_else(|| invalid("重复日期溢出"))?;
                let day = rule
                    .month_day
                    .unwrap()
                    .min((next - start).num_days() as u32);
                let d = start + Duration::days((day - 1).into());
                if d >= candidate {
                    break d;
                }
                m += 1;
                if m == 13 {
                    m = 1;
                    y += 1;
                }
            }
        }
        _ => return Err(invalid("重复类型无效")),
    };
    if rule.end.is_some_and(|end| result > end) {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}
pub fn reminder_times(draft: &TaskDraft, zone: Tz) -> AppResult<Vec<(u16, DateTime<Utc>)>> {
    let Some(due) = draft.due_date else {
        return Ok(vec![]);
    };
    draft
        .reminder_days
        .iter()
        .map(|n| {
            let day = due
                .checked_sub_signed(Duration::days((*n).into()))
                .ok_or_else(|| invalid("提醒日期溢出"))?;
            Ok((*n, local_instant(day, draft.reminder_time, zone)?))
        })
        .collect()
}
