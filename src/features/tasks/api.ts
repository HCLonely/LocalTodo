import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {Snapshot,View,TaskDraft,Settings,Task} from './types';
export interface Query {view:View;selected_date:string|null;search:string;priority:number|null;offset:number;limit:number}
async function call<T>(command:string,args?:Record<string,unknown>):Promise<T>{try{return await invoke<T>(command,args);}catch(e){throw new Error(typeof e==='object'&&e&&'message' in e?String(e.message):typeof e==='string'?e:'请通过桌面程序打开，浏览器预览不连接本地数据库。');}}
export const api={
 snapshot:(query:Query)=>call<Snapshot>('snapshot',{query}),
 save:(id:string|null,draft:TaskDraft,scope:'single'|'following')=>call<Task>('save_task',{id,draft,scope}),
 complete:(id:string,value:boolean)=>call<void>('set_completed',{id,value}),
 remove:(id:string,value:boolean)=>call<void>('set_deleted',{id,value}),
 settings:(settings:Settings)=>call<void>('update_settings',{settings}),
 export:(kind:'json'|'sqlite')=>call<string|null>('export_backup',{kind}),
 restore:()=>call<number|null>('restore_backup'),
 testNotification:()=>call<void>('test_notification'),
 markRead:()=>call<void>('mark_inbox_read'),
 subscribe:(event:string,callback:(payload?:unknown)=>void)=>listen(event,message=>callback(message.payload)),
};
