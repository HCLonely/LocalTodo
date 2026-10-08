import { useEffect, useRef } from 'react';
export default function Modal({children,onClose,label}:{children:React.ReactNode;onClose:()=>void;label:string}){
 const ref=useRef<HTMLDivElement>(null);
 const closeRef=useRef(onClose);closeRef.current=onClose;
 useEffect(()=>{const previous=document.activeElement as HTMLElement|null;const el=ref.current;el?.querySelector<HTMLElement>('button,input,select')?.focus();const key=(e:KeyboardEvent)=>{if(e.key==='Escape'){e.preventDefault();e.stopPropagation();closeRef.current();}if(e.key==='Tab'&&el){const focusable=Array.from(el.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]'));const first=focusable[0],last=focusable.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus();}}};document.addEventListener('keydown',key);return()=>{document.removeEventListener('keydown',key);previous?.focus();};},[]);
 return <div className="modal-backdrop"><div className="modal" ref={ref} role="dialog" aria-modal="true" aria-label={label}>{children}</div></div>;
}
