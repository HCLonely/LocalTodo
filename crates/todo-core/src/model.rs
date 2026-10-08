use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("任务不存在")]
    NotFound,
    #[error("本地数据库操作失败：{0}")]
    Database(#[from] rusqlite::Error),
    #[error("文件读写失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("数据格式错误：{0}")]
    Json(#[from] serde_json::Error),
}
pub type AppResult<T> = Result<T, AppError>;
pub fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subtask {
    pub title: String,
    pub completed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurrenceRule {
    pub kind: String,
    pub start: NaiveDate,
    pub end: Option<NaiveDate>,
    pub weekday: Option<u32>,
    pub month_day: Option<u32>,
}
impl RecurrenceRule {
    pub fn validate(&self) -> AppResult<()> {
        valid_date(self.start)?;
        if let Some(end) = self.end {
            valid_date(end)?;
            if end < self.start {
                return Err(invalid("重复结束日期不能早于开始日期"));
            }
        }
        match self.kind.as_str() {
            "daily" => (),
            "weekly" if self.weekday.is_some_and(|d| d <= 6) => (),
            "monthly" if self.month_day.is_some_and(|d| (1..=31).contains(&d)) => (),
            _ => return Err(invalid("重复规则无效，请检查频率和日期")),
        }
        Ok(())
    }
}
pub fn valid_date(date: NaiveDate) -> AppResult<()> {
    if (1900..=9998).contains(&date.year()) {
        Ok(())
    } else {
        Err(invalid("日期必须在1900至9998年之间"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TaskDraft {
    pub title: String,
    pub note: String,
    pub priority: u8,
    pub planned_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub reminder_days: Vec<u16>,
    pub reminder_time: NaiveTime,
    pub subtasks: Vec<Subtask>,
    pub recurrence: Option<RecurrenceRule>,
}
impl Default for TaskDraft {
    fn default() -> Self {
        Self {
            title: String::new(),
            note: String::new(),
            priority: 1,
            planned_date: None,
            due_date: None,
            due_time: None,
            reminder_days: vec![],
            reminder_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            subtasks: vec![],
            recurrence: None,
        }
    }
}
impl TaskDraft {
    pub fn validate(&mut self) -> AppResult<()> {
        self.title = self.title.trim().into();
        if self.title.is_empty() || self.title.chars().count() > 200 {
            return Err(invalid("标题不能为空，且最多200个字符"));
        }
        if self.note.chars().count() > 20000 || self.priority > 2 {
            return Err(invalid("备注最多20000个字符，优先级必须为低、普通或高"));
        }
        for date in [self.due_date, self.planned_date].into_iter().flatten() {
            valid_date(date)?;
        }
        if self.due_time.is_some() && self.due_date.is_none() {
            return Err(invalid("截止时刻需要截止日期"));
        }
        self.reminder_days.sort_unstable();
        self.reminder_days.dedup();
        if self.reminder_days.iter().any(|d| *d > 365) {
            return Err(invalid("提醒提前天数必须为0至365"));
        }
        if !self.reminder_days.is_empty() {
            if self.due_date.is_none() {
                return Err(invalid("提醒需要截止日期"));
            }
            if self.due_time.is_some_and(|t| self.reminder_time > t) {
                return Err(invalid("提醒时刻不能晚于截止时刻"));
            }
        }
        if self.subtasks.len() > 200 {
            return Err(invalid("每个任务最多200个子任务"));
        }
        for s in &mut self.subtasks {
            s.title = s.title.trim().into();
            if s.title.is_empty() || s.title.chars().count() > 200 {
                return Err(invalid("子任务标题不能为空且最多200个字符"));
            }
        }
        if let Some(r) = &self.recurrence {
            r.validate()?;
            if self.due_date.is_none() {
                return Err(invalid("重复任务需要截止日期"));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub draft: TaskDraft,
    pub completed: bool,
    pub deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub series_id: Option<String>,
    pub occurrence_date: Option<NaiveDate>,
    pub revision: u32,
}
impl Task {
    pub fn new(mut draft: TaskDraft, now: DateTime<Utc>) -> AppResult<Self> {
        draft.validate()?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            draft,
            completed: false,
            deleted: false,
            created_at: now,
            updated_at: now,
            completed_at: None,
            series_id: None,
            occurrence_date: None,
            revision: 1,
        })
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupView {
    #[default]
    Main,
    Card,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub timezone: String,
    pub default_reminder_time: NaiveTime,
    pub autostart: bool,
    #[serde(default)]
    pub startup_view: StartupView,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            timezone: iana_time_zone::get_timezone()
                .ok()
                .filter(|v| v.parse::<chrono_tz::Tz>().is_ok())
                .unwrap_or_else(|| "Asia/Shanghai".into()),
            default_reminder_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            autostart: false,
            startup_view: StartupView::Main,
        }
    }
}
impl Settings {
    pub fn zone(&self) -> AppResult<chrono_tz::Tz> {
        self.timezone
            .parse()
            .map_err(|_| invalid("时区无效，请使用IANA时区名称"))
    }
    pub fn validate(&self) -> AppResult<()> {
        self.zone()?;
        if !["light", "dark", "system"].contains(&self.theme.as_str()) {
            return Err(invalid("主题无效"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub id: String,
    #[serde(default)]
    pub root_id: Option<String>,
    #[serde(default)]
    pub excluded_dates: std::collections::BTreeSet<NaiveDate>,
    pub template: TaskDraft,
    pub rule: RecurrenceRule,
    pub cursor: Option<NaiveDate>,
    pub active: bool,
    #[serde(default)]
    pub reminder_not_before: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationBatch {
    pub id: String,
    pub title: String,
    pub body: String,
    pub task_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub submitted: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub retry_exhausted: bool,
    pub read: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationProgress {
    pub created: usize,
    pub has_more: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub tasks: Vec<Task>,
    pub total: usize,
    pub counts: std::collections::BTreeMap<String, usize>,
    pub overdue_ids: Vec<String>,
    pub inbox: Vec<NotificationBatch>,
    pub settings: Settings,
    pub today: NaiveDate,
    pub generation_pending: bool,
    pub scheduler_error: Option<String>,
}
pub struct BackgroundReport {
    pub generation: GenerationProgress,
    pub submitted: usize,
    pub next_wake_millis: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub view: String,
    pub selected_date: Option<NaiveDate>,
    pub search: String,
    pub priority: Option<u8>,
    pub offset: usize,
    pub limit: usize,
}
