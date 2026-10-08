import { confirmAction } from './confirm';
import type {Task} from '../features/tasks/types';
import { useState } from 'react';
import { CalendarDays, Repeat2, Bell, ListChecks, Sun, Trash2, Undo2, CheckCheck } from 'lucide-react';
export interface TaskListProps {tasks:Task[];overdueIds:string[];view:string;onEdit:(task:Task)=>void;onToggle:(task:Task)=>Promise<void>;onToday:(task:Task)=>Promise<void>;onDelete:(task:Task)=>Promise<void>}
export default function TaskList(props:TaskListProps){
 const [pending,setPending]=useState<string[]>([]);const [error,setError]=useState('');
 async function run(task:Task,action:(t:Task)=>Promise<void>){setError('');setPending(p=>[...p,task.id]);try{await action(task);}catch(e){setError(e instanceof Error?e.message:String(e));}finally{setPending(p=>p.filter(id=>id!==task.id));}}
 return <div className="task-list">{error&&<p role="alert" className="error">{error}</p>}{props.tasks.map(task=>{
 const overdue=props.overdueIds.includes(task.id);const children=task.draft.subtasks;const disabled=pending.includes(task.id);
 return <article className={`task-row ${task.completed?'is-completed':''}`} key={task.id}>
   <span className={`priority-line priority-${task.draft.priority}`}/>
   <input type="checkbox" className="task-check" aria-label={`${task.completed?'恢复':'完成'} ${task.draft.title}`} checked={task.completed} disabled={disabled||task.deleted} onChange={async()=>{if(!task.completed&&children.some(s=>!s.completed)&&!await confirmAction('完成父任务将同时完成所有未完成子任务，是否继续？'))return;void run(task,props.onToggle);}}/>
   <div className="task-content"><button className="task-title" onClick={()=>props.onEdit(task)} disabled={disabled}>{task.draft.title}</button>{task.draft.note&&<p className="task-note">{task.draft.note}</p>}
   <div className="task-meta">{task.draft.due_date&&<span className={overdue?'due overdue':'due'}><CalendarDays size={13}/>{task.draft.due_date}{task.draft.due_time&&` ${task.draft.due_time.slice(0,5)}`}{overdue&&<b>已逾期</b>}</span>}{task.draft.priority===2&&<span className="priority-badge">高优先级</span>}{task.draft.recurrence&&<span><Repeat2 size={13}/>{{daily:'每天',weekly:'每周',monthly:'每月'}[task.draft.recurrence.kind]}</span>}{task.draft.reminder_days.length>0&&<span><Bell size={13}/>{task.draft.reminder_days.length}次提醒</span>}{children.length>0&&<span><ListChecks size={13}/>{children.filter(s=>s.completed).length}/{children.length}</span>}{task.draft.planned_date&&<span><Sun size={13}/>{task.draft.planned_date}</span>}</div></div>
   <div className="row-actions">{!task.deleted&&!task.completed&&<button className="icon-button" title="安排到今天" aria-label={`安排到今天 ${task.draft.title}`} disabled={disabled} onClick={()=>void run(task,props.onToday)}><Sun size={16}/></button>}<button className="icon-button" title={task.deleted?'恢复任务':'移入回收站'} aria-label={`${task.deleted?'恢复任务':'删除'} ${task.draft.title}`} disabled={disabled} onClick={()=>void run(task,props.onDelete)}>{task.deleted?<Undo2 size={16}/>:<Trash2 size={16}/>}</button></div>
 </article>;})}{props.tasks.length===0&&<div className="empty-state"><div className="empty-icon"><CheckCheck size={30}/></div><h3>这里清清爽爽</h3><p>{props.view==='completed'?'完成的任务会出现在这里。':props.view==='trash'?'删除的任务会保留在这里，随时可以恢复。':'没有匹配的任务。添加一件想完成的小事吧。'}</p></div>}</div>;
}
