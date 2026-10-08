import { confirmAction } from './confirm';
import { useEffect, useRef, useState } from 'react';
import { X, Plus, Trash2, Repeat2, Bell, CalendarDays, ListChecks, Save } from 'lucide-react';
import { emptyDraft, type Task, type TaskDraft } from '../features/tasks/types';
export interface EditorProps { task: Task|null; today: string; initialDate?:string; defaultReminderTime?:string; timezone?:string; onSave: (draft:TaskDraft,scope:'single'|'following')=>Promise<void>; onClose:()=>void; onGuardChange?:(guard:()=>Promise<boolean>)=>void }
export default function TaskEditor({task,today,initialDate,defaultReminderTime,timezone,onSave,onClose,onGuardChange}:EditorProps) {
  const [draft,setDraft]=useState<TaskDraft>(()=>task?structuredClone(task.draft):{...emptyDraft(today,defaultReminderTime),due_date:initialDate||null});
  const [scope,setScope]=useState<'single'|'following'>('single');
  const [reminders,setReminders]=useState(draft.reminder_days.length>0);
  const [days,setDays]=useState(draft.reminder_days.length?draft.reminder_days.join(','):'0,1,3');
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const [newSubtask,setNewSubtask]=useState('');const titleRef=useRef<HTMLInputElement>(null);
  const original=useRef(JSON.stringify({draft,reminders,days,scope,newSubtask}));
  const set=<K extends keyof TaskDraft>(key:K,value:TaskDraft[K])=>setDraft(d=>({...d,[key]:value}));
  const confirming=useRef(false);
  const canLeave=async()=>{if(busy||confirming.current)return false;confirming.current=true;try{return (JSON.stringify({draft,reminders,days,scope,newSubtask})===original.current||await confirmAction('尚有未保存的修改，是否放弃？'));}finally{confirming.current=false;}};
  const close=async()=>{if(await canLeave())onClose();};
  useEffect(()=>{onGuardChange?.(canLeave);});
  useEffect(()=>{titleRef.current?.focus();},[]);
  useEffect(()=>{const key=(e:KeyboardEvent)=>{if(e.key==='Escape'){e.preventDefault();close();}};window.addEventListener('keydown',key);return()=>window.removeEventListener('keydown',key);});
  async function submit(e:React.FormEvent){
    e.preventDefault();setError('');
    if(!draft.title.trim()){setError('请填写任务标题');titleRef.current?.focus();return;}
    if([...draft.title.trim()].length>200){setError('标题最多200个字符');return;}
    let offsets:number[]=[];
    if(reminders){
      if(!draft.due_date){setError('请先设置截止日期，再开启提醒');return;}
      const tokens=days.split(/[,，\s]+/).filter(Boolean);
      if(!tokens.length||tokens.some(v=>!/^\d+$/.test(v)||Number(v)>365)){setError('提前天数请输入0–365的整数，用逗号分隔');return;}
      offsets=[...new Set(tokens.map(Number))].sort((a,b)=>a-b);
      if(draft.due_time&&draft.reminder_time.slice(0,5)>draft.due_time.slice(0,5)){setError('提醒时刻不能晚于截止时刻');return;}
    }
    const next={...draft,title:draft.title.trim(),reminder_days:offsets,subtasks:newSubtask.trim()?[...draft.subtasks,{title:newSubtask.trim(),completed:false}]:draft.subtasks};
    if(next.recurrence&&!next.due_date){setError('重复任务需要设置截止日期');return;}
    setBusy(true);try{await onSave(next,scope);}catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);}
  }
  const previewTokens=days.split(/[,，\s]+/).filter(Boolean);
  const previewDates=reminders&&draft.due_date&&previewTokens.length&&previewTokens.every(v=>/^\d+$/.test(v)&&Number(v)<=365)?[...new Set(previewTokens.map(Number))].sort((a,b)=>b-a).map(offset=>{const date=new Date(draft.due_date!+'T12:00:00Z');date.setUTCDate(date.getUTCDate()-offset);return Number.isNaN(date.getTime())?'':date.toISOString().slice(0,10);}):[];
  function addSubtask(){if(!newSubtask.trim())return;set('subtasks',[...draft.subtasks,{title:newSubtask.trim(),completed:false}]);setNewSubtask('');}
  return <aside className="editor" aria-label={task?'编辑任务':'新建任务'}>
    <div className="panel-heading"><div><span className="eyebrow">TASK DETAILS</span><h2>{task?'编辑任务':'新建任务'}</h2></div><button className="icon-button" onClick={close} aria-label="关闭编辑" disabled={busy}><X size={20}/></button></div>
    <form onSubmit={submit} noValidate>
      <div className="editor-body">
        <label className="field">任务标题<input ref={titleRef} value={draft.title} onChange={e=>set('title',e.target.value)} placeholder="接下来，想完成什么？" maxLength={400}/></label>
        <label className="field">备注<textarea value={draft.note} onChange={e=>set('note',e.target.value)} rows={3} placeholder="补充细节，或写下第一步…" maxLength={20000}/></label>
        <div className="field-pair"><label className="field">优先级<select aria-label="优先级" value={draft.priority} onChange={e=>set('priority',Number(e.target.value))}><option value={0}>低优先级</option><option value={1}>普通优先级</option><option value={2}>高优先级</option></select></label><label className="field">计划日期<input type="date" value={draft.planned_date||''} onChange={e=>set('planned_date',e.target.value||null)}/></label></div>
        <div className="section-title"><CalendarDays size={16}/><h3>截止日期</h3></div>
        <div className="field-pair"><label className="field">截止日期<input type="date" value={draft.due_date||''} onChange={e=>set('due_date',e.target.value||null)}/></label><label className="field">截止时刻<input type="time" value={draft.due_time?.slice(0,5)||''} onChange={e=>set('due_time',e.target.value?e.target.value+':00':null)}/></label></div>
        <p className="helper">不填时刻表示当天结束前。计划日期不会改变截止日期。</p>
        <div className="section-title"><Repeat2 size={16}/><h3>重复任务</h3></div>
        <label className="field">重复频率<select aria-label="重复频率" value={draft.recurrence?.kind||'none'} onChange={e=>{const kind=e.target.value;if(kind==='none'){set('recurrence',null);return;}const start=draft.due_date||today;set('due_date',start);set('recurrence',{kind:kind as 'daily'|'weekly'|'monthly',start,end:null,weekday:(new Date(start+'T12:00:00').getDay()+6)%7,month_day:Number(start.slice(-2))});}}><option value="none">不重复</option><option value="daily">每天</option><option value="weekly">每周</option><option value="monthly">每月</option></select></label>
        {draft.recurrence&&<div className="repeat-details">
          <div className="field-pair"><label className="field">开始日期<input type="date" value={draft.recurrence.start} onChange={e=>set('recurrence',{...draft.recurrence!,start:e.target.value})}/></label><label className="field">结束日期（可选）<input type="date" min={draft.recurrence.start} value={draft.recurrence.end||''} onChange={e=>set('recurrence',{...draft.recurrence!,end:e.target.value||null})}/></label></div>
          {draft.recurrence.kind==='weekly'&&<label className="field">每周哪一天<select aria-label="每周哪一天" value={draft.recurrence.weekday??0} onChange={e=>set('recurrence',{...draft.recurrence!,weekday:Number(e.target.value)})}>{['周一','周二','周三','周四','周五','周六','周日'].map((v,i)=><option key={v} value={i}>{v}</option>)}</select></label>}
          {draft.recurrence.kind==='monthly'&&<label className="field">每月几号<input type="number" min={1} max={31} value={draft.recurrence.month_day??1} onChange={e=>set('recurrence',{...draft.recurrence!,month_day:Number(e.target.value)})}/><span className="helper">短月取月末，之后仍按原日号生成。</span></label>}
          <p className="helper">每期独立完成，未完成的旧任务保留为逾期。</p>
        </div>}
        {task?.series_id&&<label className="field">修改范围<select aria-label="修改范围" value={scope} onChange={e=>setScope(e.target.value as 'single'|'following')}><option value="single">仅本次任务</option><option value="following">本次及以后的未完成任务</option></select></label>}
        <div className="section-title"><Bell size={16}/><h3>截止提醒</h3><label className="switch-label"><input type="checkbox" aria-label="开启截止提醒" checked={reminders} onChange={e=>setReminders(e.target.checked)}/><span>{reminders?'已开启':'未开启'}</span></label></div>
        {reminders&&<div className="repeat-details"><div className="field-pair"><label className="field">提前天数<input value={days} onChange={e=>setDays(e.target.value)} placeholder="0,1,3"/></label><label className="field">提醒时刻<input type="time" value={draft.reminder_time.slice(0,5)} onChange={e=>set('reminder_time',e.target.value+':00')}/></label></div><p className="helper">0 表示截止当天；多个天数用逗号分隔。错过的提醒恢复后合并通知。</p>{previewDates.length>0&&<p className="helper" aria-label="提醒日期预览">将在 {previewDates.slice(0,6).join("、")}{previewDates.length>6?` 等 ${previewDates.length} 个日期`:""} 的 {draft.reminder_time.slice(0,5)} 提醒（{timezone||"应用时区"}）。重复任务按每期截止日期重新计算。</p>}</div>}
        <div className="section-title"><ListChecks size={16}/><h3>子任务</h3><span className="muted">{draft.subtasks.filter(t=>t.completed).length}/{draft.subtasks.length}</span></div>
        <div className="subtask-list">{draft.subtasks.map((sub,i)=><div className="subtask" key={i}><input type="checkbox" aria-label={`完成子任务 ${sub.title}`} checked={sub.completed} onChange={e=>set('subtasks',draft.subtasks.map((s,j)=>j===i?{...s,completed:e.target.checked}:s))}/><input aria-label={`子任务 ${i+1}`} value={sub.title} onChange={e=>set('subtasks',draft.subtasks.map((s,j)=>j===i?{...s,title:e.target.value}:s))}/><button type="button" className="icon-button" aria-label={`删除子任务 ${sub.title}`} onClick={()=>set('subtasks',draft.subtasks.filter((_,j)=>j!==i))}><Trash2 size={15}/></button></div>)}</div>
        <div className="subtask-add"><input aria-label="新子任务" value={newSubtask} onChange={e=>setNewSubtask(e.target.value)} placeholder="添加一个小步骤" onKeyDown={e=>{if(e.key==='Enter'){e.preventDefault();addSubtask();}}}/><button type="button" aria-label="添加子任务" className="icon-button" onClick={addSubtask}><Plus size={18}/></button></div>
        {error&&<p className="error" role="alert">{error}</p>}
      </div>
      <footer className="editor-footer"><button type="button" className="button secondary" disabled={busy} onClick={close}>取消</button><button className="button primary" type="submit" disabled={busy}><Save size={16}/>{busy?'保存中…':'保存任务'}</button></footer>
    </form>
  </aside>;
}
