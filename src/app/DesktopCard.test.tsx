import {render,screen,waitFor} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {it,expect,vi} from 'vitest';
import DesktopCard from './DesktopCard';
import {emptyDraft} from '../features/tasks/types';
const mock=vi.hoisted(()=>({snapshot:vi.fn(),save:vi.fn(),complete:vi.fn(),cardPin:vi.fn().mockResolvedValue(true),setCardPin:vi.fn(),hideCard:vi.fn(),openMain:vi.fn(),editInMain:vi.fn(),subscribe:vi.fn().mockResolvedValue(()=>{})}));
vi.mock('../features/tasks/api',()=>({api:mock}));
const snapshot={tasks:[],total:0,today:'2026-10-08',overdue_ids:[],counts:{today:0,overdue:0},settings:{theme:'light'},generation_pending:false};
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
