import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe,it,expect,vi } from 'vitest';
import TaskEditor from '../../app/TaskEditor';
import { emptyDraft, type Task } from './types';

describe('任务编辑',()=>{
  it('仅修改提醒或待添加子任务也必须确认放弃',async()=>{
    const user=userEvent.setup();const close=vi.fn();const confirm=vi.spyOn(window,'confirm').mockReturnValue(false);
    render(<TaskEditor task={null} today="2026-10-08" onSave={async()=>{}} onClose={close}/>);
    await user.click(screen.getByLabelText('开启截止提醒'));await user.click(screen.getByRole('button',{name:'关闭编辑'}));
    expect(confirm).toHaveBeenCalled();expect(close).not.toHaveBeenCalled();
    await user.click(screen.getByLabelText('开启截止提醒'));confirm.mockClear();
    await user.type(screen.getByLabelText('新子任务'),'还没有添加的步骤');await user.click(screen.getByRole('button',{name:'取消'}));
    expect(confirm).toHaveBeenCalled();expect(close).not.toHaveBeenCalled();confirm.mockRestore();
  });
  it('拒绝空白标题并保留输入直到保存成功',async()=>{
    const user=userEvent.setup();const save=vi.fn().mockRejectedValue(new Error('磁盘空间不足'));
    render(<TaskEditor task={null} today="2026-10-08" onSave={save} onClose={()=>{}}/>);
    await user.click(screen.getByRole('button',{name:'保存任务'}));expect(save).not.toHaveBeenCalled();
    await user.type(screen.getByLabelText('任务标题'),'提交报告');await user.click(screen.getByRole('button',{name:'保存任务'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('磁盘空间不足');expect(screen.getByLabelText('任务标题')).toHaveValue('提交报告');
  });
  it('阻止无截止日期的提醒并支持多个提前天数',async()=>{
    const user=userEvent.setup();const save=vi.fn().mockResolvedValue(undefined);
    render(<TaskEditor task={null} today="2026-10-08" onSave={save} onClose={()=>{}}/>);
    await user.type(screen.getByLabelText('任务标题'),'提醒任务');await user.click(screen.getByLabelText('开启截止提醒'));
    await user.click(screen.getByRole('button',{name:'保存任务'}));expect(screen.getByRole('alert')).toHaveTextContent('截止日期');expect(save).not.toHaveBeenCalled();
    await user.type(screen.getByLabelText('截止日期'),'2026-10-20');
    expect(screen.getByLabelText('提醒日期预览')).toHaveTextContent('2026-10-17、2026-10-19、2026-10-20');
    expect(screen.getByLabelText('提醒日期预览')).toHaveTextContent('09:00');
    await user.click(screen.getByRole('button',{name:'保存任务'}));
    expect(save).toHaveBeenCalledWith(expect.objectContaining({due_date:'2026-10-20',reminder_days:[0,1,3]}),'single');
  });
  it('编辑重复实例时明确选择本次及以后',async()=>{
    const user=userEvent.setup();const save=vi.fn().mockResolvedValue(undefined);
    const task:Task={id:'t',draft:{...emptyDraft('2026-10-08'),title:'读书',due_date:'2026-10-08',recurrence:{kind:'daily',start:'2026-10-08',end:null,weekday:null,month_day:null}},completed:false,deleted:false,series_id:'s',occurrence_date:'2026-10-08',revision:1,created_at:'',updated_at:'',completed_at:null};
    render(<TaskEditor task={task} today="2026-10-08" onSave={save} onClose={()=>{}}/>);
    await user.selectOptions(screen.getByLabelText('修改范围'),'following');await user.click(screen.getByRole('button',{name:'保存任务'}));expect(save).toHaveBeenCalledWith(expect.anything(),'following');
  });
});
