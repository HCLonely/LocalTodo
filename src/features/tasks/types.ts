export type View = 'today'|'week'|'month'|'date'|'inbox'|'overdue'|'completed'|'trash'|'all';
export interface Subtask { title: string; completed: boolean }
export interface RecurrenceRule { kind: 'daily'|'weekly'|'monthly'; start: string; end: string|null; weekday: number|null; month_day: number|null }
export interface TaskDraft {
  title: string; note: string; priority: number; planned_date: string|null;
  due_date: string|null; due_time: string|null; reminder_days: number[];
  reminder_time: string; subtasks: Subtask[]; recurrence: RecurrenceRule|null;
}
export interface Task { id: string; draft: TaskDraft; completed: boolean; deleted: boolean; created_at: string; updated_at: string; completed_at: string|null; series_id: string|null; occurrence_date: string|null; revision: number }
export interface Settings { theme: 'light'|'dark'|'system'; timezone: string; default_reminder_time: string; autostart: boolean; startup_view?: 'main'|'card' }
export interface ReminderEntry { id: string; title: string; body: string; task_ids: string[]; created_at: string; submitted: boolean; error: string|null; retry_exhausted?:boolean; read: boolean }
export interface Snapshot { tasks: Task[]; total: number; counts: Record<View,number>; overdue_ids:string[]; inbox: ReminderEntry[]; settings: Settings; today: string; generation_pending: boolean; scheduler_error?:string|null }
export function emptyDraft(today: string, reminderTime='09:00:00'): TaskDraft { return {title:'',note:'',priority:1,planned_date:null,due_date:null,due_time:null,reminder_days:[],reminder_time:reminderTime,subtasks:[],recurrence:null} }
