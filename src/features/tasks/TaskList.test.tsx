import {render,screen} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {it,expect,vi} from 'vitest';
import TaskList from '../../app/TaskList';
import {emptyDraft,type Task} from './types';
const task:Task={id:'id1',draft:{...emptyDraft('2026-10-08'),title:'写报告',due_date:'2026-10-12'},completed:false,deleted:false,created_at:'',updated_at:'',completed_at:null,series_id:null,occurrence_date:null,revision:1};
it('完成写入失败时保留未完成状态并展示错误',async()=>{
 render(<TaskList tasks={[task]} overdueIds={[]} view="today" onEdit={()=>{}} onToggle={async()=>{throw new Error('保存失败')}} onToday={async()=>{}} onDelete={async()=>{}}/>);
 await userEvent.click(screen.getByRole('checkbox',{name:'完成 写报告'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('保存失败');expect(screen.getByRole('checkbox')).not.toBeChecked();
});
it('逾期使用文字标识且点击标题打开同一任务',async()=>{
 const edit=vi.fn();render(<TaskList tasks={[task]} overdueIds={['id1']} view="overdue" onEdit={edit} onToggle={async()=>{}} onToday={async()=>{}} onDelete={async()=>{}}/>);
 expect(screen.getByText('已逾期')).toBeVisible();await userEvent.click(screen.getByRole('button',{name:'写报告'}));expect(edit).toHaveBeenCalledWith(task);
 expect(screen.getByRole('article')).toHaveClass('is-overdue');
});
it('本日任务中的已完成任务保持勾选和完成样式并支持恢复',async()=>{
 const completed={...task,completed:true};const toggle=vi.fn().mockResolvedValue(undefined);
 render(<TaskList tasks={[completed]} overdueIds={[]} view="today" onEdit={()=>{}} onToggle={toggle} onToday={async()=>{}} onDelete={async()=>{}}/>);
 const checkbox=screen.getByRole('checkbox',{name:'恢复 写报告'});
 expect(checkbox).toBeChecked();expect(screen.getByRole('article')).toHaveClass('is-completed');
 await userEvent.click(checkbox);expect(toggle).toHaveBeenCalledWith(completed);
});
