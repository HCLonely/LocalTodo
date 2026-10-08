import {render,screen} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {it,expect,vi} from 'vitest';
import Settings from './Settings';
vi.mock('../features/tasks/api',()=>({api:{settings:vi.fn().mockResolvedValue(undefined)}}));
it('设置输入更新时焦点保持在输入框，不跳回关闭按钮',async()=>{
 const user=userEvent.setup();render(<Settings settings={{theme:'light',timezone:'Asia/Shanghai',default_reminder_time:'09:00:00',autostart:false}} onClose={()=>{}} onChanged={async()=>{}}/>);
 const input=screen.getByLabelText('应用时区');await user.clear(input);await user.type(input,'America/New_York');expect(input).toHaveValue('America/New_York');expect(input).toHaveFocus();
});
