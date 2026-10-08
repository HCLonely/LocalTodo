import {render,screen,waitFor} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {it,expect,vi} from 'vitest';
import App from './App';
import {emptyDraft,type Snapshot} from '../features/tasks/types';
const mock=vi.hoisted(()=>({snapshot:vi.fn(),save:vi.fn(),complete:vi.fn(),remove:vi.fn(),settings:vi.fn(),export:vi.fn(),restore:vi.fn(),testNotification:vi.fn(),markRead:vi.fn(),subscribe:vi.fn().mockResolvedValue(()=>{})}));
vi.mock('../features/tasks/api',()=>({api:mock}));
const data:Snapshot={tasks:[],total:0,counts:{today:0,week:0,month:0,date:0,inbox:0,overdue:0,completed:0,trash:0,all:0},overdue_ids:[],inbox:[],settings:{theme:'light',timezone:'Asia/Shanghai',default_reminder_time:'09:00:00',autostart:false},today:'2026-10-08',generation_pending:false};
it('新建后刷新真实服务结果并在本日与本月共享同一任务',async()=>{
 const user=userEvent.setup();let created=false;
 mock.snapshot.mockImplementation(async()=>({...data,total:created?1:0,tasks:created?[{id:'shared',draft:{...emptyDraft('2026-10-08'),title:'新任务',due_date:'2026-10-08'},completed:false,deleted:false,series_id:null,occurrence_date:null,revision:1,created_at:'',updated_at:'',completed_at:null}]:[]}));
 mock.save.mockImplementation(async()=>{created=true;});
 render(<App/>);await screen.findByRole('heading',{name:'本日任务'});
 await user.click(screen.getByRole('button',{name:'新建任务'}));await user.type(screen.getByLabelText('任务标题'),'新任务');await user.click(screen.getByRole('button',{name:'保存任务'}));
 await screen.findByRole('button',{name:'新任务'});expect(screen.queryByLabelText('任务标题')).not.toBeInTheDocument();
 await user.click(screen.getByRole('button',{name:/本月任务/}));expect(await screen.findByRole('button',{name:'新任务'})).toBeVisible();
 await waitFor(()=>expect(mock.snapshot).toHaveBeenLastCalledWith(expect.objectContaining({view:'month'})));
});
