import {useEffect,useLayoutEffect,useRef} from 'react';
import {createPortal} from 'react-dom';

export interface ContextMenuItem {label:string;onSelect:()=>void;disabled?:boolean;danger?:boolean}
export default function ContextMenu({x,y,items,onClose}:{x:number;y:number;items:ContextMenuItem[];onClose:()=>void}){
 const ref=useRef<HTMLDivElement>(null);const closeRef=useRef(onClose);closeRef.current=onClose;
 useLayoutEffect(()=>{const menu=ref.current;if(!menu)return;const box=menu.getBoundingClientRect();menu.style.left=Math.max(6,Math.min(x,window.innerWidth-box.width-6))+'px';menu.style.top=Math.max(6,Math.min(y,window.innerHeight-box.height-6))+'px';menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();},[x,y]);
 useEffect(()=>{const outside=(event:PointerEvent)=>{if(!ref.current?.contains(event.target as Node))closeRef.current();};const dismiss=()=>closeRef.current();document.addEventListener('pointerdown',outside,true);window.addEventListener('blur',dismiss);window.addEventListener('resize',dismiss);return()=>{document.removeEventListener('pointerdown',outside,true);window.removeEventListener('blur',dismiss);window.removeEventListener('resize',dismiss);};},[]);
 return createPortal(<div ref={ref} role="menu" aria-label="小卡片右键菜单" className="card-context-menu" style={{left:x,top:y}} onContextMenu={event=>event.preventDefault()} onKeyDown={event=>{
  const buttons=Array.from(ref.current?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')||[]);const index=buttons.indexOf(document.activeElement as HTMLButtonElement);
  if(event.key==='Escape'||event.key==='Tab'){event.preventDefault();event.stopPropagation();onClose();}
  if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)){event.preventDefault();const next=event.key==='Home'?0:event.key==='End'?buttons.length-1:(index+(event.key==='ArrowDown'?1:-1)+buttons.length)%buttons.length;buttons[next]?.focus();}
 }}>{items.map(item=><button key={item.label} role="menuitem" tabIndex={-1} disabled={item.disabled} className={item.danger?'danger':''} onClick={()=>{onClose();item.onSelect();}}>{item.label}</button>)}</div>,document.body);
}
