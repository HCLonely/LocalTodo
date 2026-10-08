import {it,expect,vi} from 'vitest';
import {confirmAction} from './confirm';
const dialog=vi.hoisted(()=>vi.fn());
vi.mock('@tauri-apps/plugin-dialog',()=>({confirm:dialog}));
it('等待原生异步取消结果，失败时也不允许破坏性操作',async()=>{
 Object.defineProperty(window,'__TAURI_INTERNALS__',{value:{},configurable:true});
 try {
  dialog.mockResolvedValue(false);expect(await confirmAction('放弃修改？')).toBe(false);
  dialog.mockRejectedValue(new Error('dialog unavailable'));expect(await confirmAction('放弃修改？')).toBe(false);
  dialog.mockResolvedValue(true);expect(await confirmAction('继续？')).toBe(true);
 } finally { delete (window as unknown as Record<string,unknown>).__TAURI_INTERNALS__; }
});
