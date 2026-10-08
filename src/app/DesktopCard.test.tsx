import {render,screen,waitFor,fireEvent} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {it,expect,vi} from 'vitest';
import DesktopCard from './DesktopCard';
import {emptyDraft} from '../features/tasks/types';
const mock=vi.hoisted(()=>({snapshot:vi.fn(),save:vi.fn(),complete:vi.fn(),remove:vi.fn(),cardPin:vi.fn().mockResolvedValue(true),setCardPin:vi.fn(),hideCard:vi.fn(),openMain:vi.fn(),editInMain:vi.fn(),subscribe:vi.fn().mockResolvedValue(()=>{})}));
vi.mock('../features/tasks/api',()=>({api:mock}));
const snapshot={tasks:[],total:0,today:'2026-10-08',overdue_ids:[],counts:{today:0,overdue:0},settings:{theme:'light'},generation_pending:false};
it('任务右键只显示管理菜单，编辑和删除使用正确任务，Esc可关闭',async()=>{
 const task={id:'context-task',draft:{...emptyDraft('2026-10-08'),title:'右键任务'},completed:false,deleted:false,series_id:null,occurrence_date:null,revision:1,created_at:'',updated_at:'',completed_at:null};
 mock.snapshot.mockResolvedValue({...snapshot,tasks:[task],total:1});mock.editInMain.mockResolvedValue(undefined);mock.remove.mockRejectedValue(new Error('删除失败'));
 const user=userEvent.setup();render(<DesktopCard/>);const title=await screen.findByRole('button',{name:'右键任务'});
 const event=new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:100,clientY:100});expect(title.dispatchEvent(event)).toBe(false);
 expect(await screen.findByRole('menu')).not.toHaveTextContent(/检查|Inspect|DevTools|调试/);
 await user.click(screen.getByRole('menuitem',{name:'编辑任务'}));expect(mock.editInMain).toHaveBeenCalledWith('context-task');
 fireEvent.contextMenu(title);await user.keyboard('{Escape}');expect(screen.queryByRole('menu')).not.toBeInTheDocument();
 fireEvent.contextMenu(title);await user.click(screen.getByRole('menuitem',{name:'移入回收站'}));expect(mock.remove).toHaveBeenCalledWith('context-task',true);expect(await screen.findByRole('alert')).toHaveTextContent('删除失败');expect(title).toBeVisible();
});
it('空白右键支持快速添加入口和置顶，点击外部关闭',async()=>{
 mock.snapshot.mockResolvedValue(snapshot);mock.setCardPin.mockResolvedValue(false);const user=userEvent.setup();render(<DesktopCard/>);await screen.findByText('今天清清爽爽');
 fireEvent.contextMenu(screen.getByRole('heading',{name:'一步一步，完成今天。'}));await user.click(screen.getByRole('menuitem',{name:'添加今日任务'}));expect(screen.getByLabelText('快速添加任务')).toHaveFocus();
 fireEvent.contextMenu(screen.getByRole('heading'));await user.click(screen.getByRole('menuitem',{name:'取消置顶'}));expect(mock.setCardPin).toHaveBeenCalledWith(false);
 fireEvent.contextMenu(screen.getByRole('heading'));fireEvent.pointerDown(document.body);expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});
it('小卡片快速添加到今日，失败保留输入，成功后刷新',async()=>{
 mock.snapshot.mockResolvedValue(snapshot);mock.save.mockRejectedValueOnce(new Error('磁盘满')).mockResolvedValueOnce({});
 const user=userEvent.setup();render(<DesktopCard/>);await screen.findByText('今天清清爽爽');
 await user.type(screen.getByLabelText('快速添加任务'),'卡片任务');await user.click(screen.getByRole('button',{name:'添加任务'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('磁盘满');expect(screen.getByLabelText('快速添加任务')).toHaveValue('卡片任务');
 await user.click(screen.getByRole('button',{name:'添加任务'}));await waitFor(()=>expect(screen.getByLabelText('快速添加任务')).toHaveValue(''));
 expect(mock.save).toHaveBeenLastCalledWith(null,expect.objectContaining({title:'卡片任务',planned_date:'2026-10-08',due_date:'2026-10-08'}),'single');
});
it('置顶失败保留原状态并显示错误',async()=>{
 mock.snapshot.mockResolvedValue(snapshot);mock.setCardPin.mockRejectedValue(new Error('置顶失败'));
 const user=userEvent.setup();render(<DesktopCard/>);const button=await screen.findByRole('button',{name:'取消置顶'});
 await user.click(button);expect(await screen.findByRole('alert')).toHaveTextContent('置顶失败');expect(button).toHaveAttribute('aria-pressed','true');
});
it('完成写入失败时卡片保留未完成状态',async()=>{
 mock.snapshot.mockResolvedValue({...snapshot,total:1,tasks:[{id:'card-task',draft:{...emptyDraft('2026-10-08'),title:'还没完成'},completed:false,deleted:false,series_id:null,occurrence_date:null,revision:1,created_at:'',updated_at:'',completed_at:null}]});
 mock.complete.mockRejectedValue(new Error('保存失败'));const user=userEvent.setup();render(<DesktopCard/>);
 const checkbox=await screen.findByRole('checkbox',{name:'完成 还没完成'});await user.click(checkbox);
 expect(await screen.findByRole('alert')).toHaveTextContent('保存失败');expect(checkbox).not.toBeChecked();
});
