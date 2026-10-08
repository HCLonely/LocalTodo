import { confirm } from '@tauri-apps/plugin-dialog';

/** Tauri dialogs are asynchronous; every destructive confirmation must await the answer. */
export async function confirmAction(message: string): Promise<boolean> {
  try {
    if ('__TAURI_INTERNALS__' in window) {
      return await confirm(message, {title:'确认操作',kind:'warning',okLabel:'继续',cancelLabel:'取消操作'});
    }
    return await window.confirm(message);
  } catch {
    // A failed dialog never grants consent to discard data.
    return false;
  }
}
