import { Bell, X, CheckCircle2, CircleAlert } from 'lucide-react';
import type {ReminderEntry} from '../tasks/types';
import Modal from '../../app/Modal';
export default function ReminderInbox({entries,onClose}:{entries:ReminderEntry[];onClose:()=>void}){
 return <Modal label="提醒中心" onClose={onClose}><header className="panel-heading"><div><span className="eyebrow">A GENTLE NUDGE</span><h2>提醒中心</h2></div><button className="icon-button" aria-label="关闭提醒中心" onClick={onClose}><X size={20}/></button></header><div className="reminder-list">{entries.length?entries.map(entry=><article key={entry.id} className="reminder-card"><div className="reminder-icon">{entry.submitted?<CheckCircle2 size={20}/>:<CircleAlert size={20}/>}</div><div><h3>{entry.title}</h3><p>{entry.body}</p><span className="helper">{new Date(entry.created_at).toLocaleString('zh-CN')} · {entry.submitted?'已提交系统通知':entry.error?'系统通知未提交':'应用内提醒'}</span>{entry.error&&<p className="error">{entry.error}。请检查通知权限，稍后自动重试。</p>}</div></article>):<div className="empty-state"><div className="empty-icon"><Bell size={28}/></div><h3>暂无提醒</h3><p>为任务设置截止提醒，到点后会在这里留下记录。</p></div>}</div></Modal>;
}
