import {render} from '@testing-library/react';
import {it,expect} from 'vitest';
import {useContextMenuPolicy} from './useContextMenuPolicy';
function Window(){useContextMenuPolicy();return <input/>;}
it('取消浏览器右键及调试快捷键，保持普通编辑快捷键可用',()=>{
 const {unmount}=render(<Window/>);
 expect(document.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true}))).toBe(false);
 expect(window.dispatchEvent(new KeyboardEvent('keydown',{key:'F12',cancelable:true}))).toBe(false);
 expect(window.dispatchEvent(new KeyboardEvent('keydown',{key:'I',ctrlKey:true,shiftKey:true,cancelable:true}))).toBe(false);
 expect(window.dispatchEvent(new KeyboardEvent('keydown',{key:'v',ctrlKey:true,cancelable:true}))).toBe(true);
 unmount();expect(document.dispatchEvent(new MouseEvent('contextmenu',{cancelable:true}))).toBe(true);
});
