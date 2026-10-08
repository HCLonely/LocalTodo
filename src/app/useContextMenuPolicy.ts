import {useEffect} from 'react';

/** Cancel browser menus without blocking the card's own context-menu handler. */
export function useContextMenuPolicy(){
 useEffect(()=>{
  const context=(event:MouseEvent)=>event.preventDefault();
  const keyboard=(event:KeyboardEvent)=>{if(event.key==='F12'||((event.ctrlKey||event.metaKey)&&event.shiftKey&&['i','j','c'].includes(event.key.toLowerCase())))event.preventDefault();};
  document.addEventListener('contextmenu',context,true);window.addEventListener('keydown',keyboard,true);
  return()=>{document.removeEventListener('contextmenu',context,true);window.removeEventListener('keydown',keyboard,true);};
 },[]);
}
