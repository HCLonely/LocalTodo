# Local Todo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现Windows本地Todo桌面程序，支持日周月与指定日期视图、重复任务、截止提醒和托盘运行。

**Architecture:** Tauri承载React界面，Rust统一处理任务规则、SQLite事务、重复实例和提醒。Rust后台调度独立于窗口显示状态，通过事件更新界面；所有持久化操作经过统一服务。

**Tech Stack:** Rust stable、Tauri 2、React、TypeScript、Vite、rusqlite、chrono/chrono-tz、serde；Vitest/Testing Library与Rust测试。

**Spec:** ../specs/2026-10-08-local-todo-design.md

## Global Constraints

- 第一版为 Windows 桌面版。
- 使用 Rust 后端与网页技术界面，采用 Tauri。
- 本日、本周、本月按截止日期自动归类，另支持安排到今天。
- 关闭窗口后驻留托盘，使用系统通知。
- 包含每日、每周、每月重复任务，数据完全本地。
- 本计划相对路径基于未来独立LocalTodo工程，不指当前Fortran工程；实现前确定工程绝对路径。
- 按日历生成重复实例，旧实例保留逾期；每月31日遇短月取月末，次月回到原日号。
- 多个提前天数提醒，默认09:00，恢复运行合并补发；中文界面包含单层子任务。
- 锁定依赖与工具链；禁止把开发模式通知结果代替安装版验收。

## Review Focus

- 跨月/跨年周、闰日、日期截止边界与夏令时：任务视图和提醒按产品日历语义一致计算（Task 2/5）。
- 长期离线与重复启动：逐批生成实例，保留旧未完成任务，无重复实例或通知轰炸（Task 4/5）。
- 到点同时完成、删除、修改：事务与提醒版本使旧任务不再发送（Task 5）。
- 睡眠、隐藏窗口、通知关闭：Rust调度继续工作，恢复补发，应用内记录保留（Task 1/5）。
- 磁盘失败、损坏导入、迁移失败：不得报告保存成功或覆盖唯一可恢复数据（Task 3/7）。

## File Structure

```text
src/app/{App.tsx,Sidebar.tsx,TaskList.tsx,TaskEditor.tsx,Settings.tsx}
src/features/tasks/{api.ts,types.ts,TaskList.test.tsx,TaskEditor.test.tsx}
src/features/recurrence/{RecurrenceEditor.tsx,RecurrenceEditor.test.tsx}
src/features/reminders/{ReminderEditor.tsx,ReminderInbox.tsx}
src/styles/theme.css
src-tauri/{Cargo.toml,tauri.conf.json,capabilities/default.json}
src-tauri/src/{main.rs,lib.rs,commands.rs,error.rs}
src-tauri/src/domain/{mod.rs,task.rs,calendar.rs,recurrence.rs,reminder.rs}
src-tauri/src/storage/{mod.rs,repository.rs,migrations.rs,backup.rs}
src-tauri/src/services/{mod.rs,tasks.rs,recurrence.rs,reminders.rs}
src-tauri/src/platform/{mod.rs,tray.rs,notifications.rs,lifecycle.rs}
src-tauri/migrations/001_initial.sql
src-tauri/tests/{calendar.rs,repository.rs,task_service.rs,recurrence.rs,reminders.rs,backup.rs}
docs/{acceptance.md,release.md}
```

## Shared Types and Interfaces

在Task 2定义：TaskId/SeriesId为UUID新类型；LocalDate为NaiveDate；AppError统一错误枚举；AppResult<T>=Result<T,AppError>。Task包含产品表中字段；TaskDraft为用户可编辑字段；TaskPatch包含可区分“不修改/清空/赋值”的字段更新；ViewQuery包含view、selected_date、priority、search、完成筛选和分页；TaskPage含items/total。

CalendarContext包含now_utc: DateTime<Utc>、zone: Tz；Clock: Send + Sync提供now_utc() -> DateTime<Utc>。实际后台和测试使用同一接口。RecurrenceSpec为Daily/Weekly { weekday }/Monthly { day }，同时含start/end、模板与时区。ReminderRule含offset_days: Vec<u16>与at: NaiveTime；ReminderBatch包含inbox_id、任务摘要和唯一投递键。

## Task 1: 安装版系统可行性

**Files:** 创建Tauri/前端基础配置、src-tauri/src/{main.rs,lib.rs}、platform/{mod.rs,tray.rs,notifications.rs,lifecycle.rs}、docs/acceptance.md。

**Interfaces:** 提供Notifier: Send + Sync，submit(&self, batch: &ReminderBatch) -> AppResult<()>；平台模块用测试批次验证。后续Task 2定义最终DTO。托盘菜单以事件唤醒主窗口。

- [ ] 建立独立工程，锁定工具链和依赖，配置仅本地内容与最小IPC权限，加入单实例。
- [ ] 实现托盘打开/退出，关闭窗口时隐藏；Rust计时器每10秒提交一次测试通知，开关仅在测试构建启用。
- [ ] 运行`npm run tauri build`，安装NSIS包；隐藏窗口验证两次通知，重复启动只激活同一进程。
- [ ] 在Windows通知允许/关闭两种设置下记录实际行为；验证隐藏窗口后后台不依赖WebView计时器。检查通知点击是否可可靠打开窗口，失败则记为后续增强。
- [ ] 通过则记录验收并提交`feat: establish installed desktop shell`；通知注册失败先修复再进入Task 2。

## Task 2: 任务类型与日历计算

**Files:** 创建domain/{mod.rs,task.rs,calendar.rs,recurrence.rs,reminder.rs}、error.rs、tests/calendar.rs。

**Interfaces:** 定义全部Shared Types；提供due_boundary(task: &Task, zone: Tz) -> AppResult<Option<DateTime<Utc>>>与matches_view(task: &Task, query: &ViewQuery, ctx: &CalendarContext) -> AppResult<bool>。

- [ ] 编写calendar测试：10月12日任务在10月8日不属于本周而属于本月，计划为10月8日则属于本日；2026-10-08全天截止在10月9日00:00逾期；跨年周仍完整查询。
- [ ] 加入闰年2月日期合法性、空白标题拒绝、提醒天数重复去重、超过365拒绝、夏令时不存在/重复时刻的具体断言。
- [ ] 运行`cargo test --manifest-path src-tauri/Cargo.toml --test calendar`确认失败。
- [ ] 实现类型与纯计算，输入时钟而非在函数内读取当前时间。
- [ ] 同命令通过后提交`feat: define calendar and task rules`。

## Task 3: 本地任务服务与持久化

**Files:** 创建storage/{mod.rs,repository.rs,migrations.rs}、migrations/001_initial.sql、services/{mod.rs,tasks.rs}、commands.rs、tests/{repository.rs,task_service.rs}；修改lib.rs。

**Interfaces:** AppService::create_task(TaskDraft) -> AppResult<Task>；update_task(TaskId, TaskPatch) -> AppResult<Task>；set_completed(TaskId, bool) -> AppResult<Task>；set_deleted(TaskId, bool) -> AppResult<()>；list_tasks(ViewQuery, CalendarContext) -> AppResult<TaskPage>。命令转发相同DTO并序列化错误。

- [ ] 编写临时数据库测试：创建重开库保留；完成与恢复正确；删除恢复正确；无截止日期任务可保存；数据库只读时保存返回错误且数据不变。
- [ ] 运行`cargo test --manifest-path src-tauri/Cargo.toml --test repository --test task_service`确认失败。
- [ ] 实现迁移、外键、WAL、busy timeout、查询索引与事务；任务修改同时递增schedule_revision并取消旧pending提醒。
- [ ] 连接Tauri命令，写入成功才发tasks_changed事件；同命令通过后提交`feat: persist local tasks transactionally`。

## Task 4: 重复模板与实例

**Files:** 创建services/recurrence.rs、tests/recurrence.rs；修改domain/recurrence.rs、repository.rs、commands.rs。

**Interfaces:** next_occurrence(spec: &RecurrenceSpec, after: LocalDate) -> AppResult<Option<LocalDate>>；AppService::save_series(RecurrenceSpec) -> AppResult<SeriesId>；materialize_until(LocalDate, limit: usize) -> AppResult<GenerationProgress>；edit_series_from(SeriesId, LocalDate, RecurrenceSpec) -> AppResult<SeriesId>。GenerationProgress含created和has_more；后台调用limit固定100。

- [ ] 测试每日未完成仍生成下一期、周规则不早于start、end包含当日；月31日生成2027-01-31/02-28/03-31及2028闰年2月29日。
- [ ] 测试同一范围重复生成created=0；单期删除不重生；离线两年分批推进每批≤100并最终无遗漏；系列拆分保留历史且取消未来旧提醒。
- [ ] 运行`cargo test --manifest-path src-tauri/Cargo.toml --test recurrence`确认失败。
- [ ] 实现模板/实例分离、唯一约束、生成游标、事务与单期/后续编辑；每期独立复制子任务。
- [ ] 同命令通过后提交`feat: generate calendar recurrence instances`。

## Task 5: 可靠提醒与后台调度

**Files:** 创建services/reminders.rs、tests/reminders.rs；修改domain/reminder.rs、platform/notifications.rs、lifecycle.rs、repository.rs、lib.rs。

**Interfaces:** build_jobs(task: &Task, rule: &ReminderRule, ctx: &CalendarContext) -> AppResult<Vec<ReminderJob>>；ReminderService::tick(ctx: CalendarContext) -> AppResult<TickReport>；dispatch经过与任务更新共用的串行服务，提交前校验revision、未完成及未删除。TickReport含submitted/failed/next_wake。

- [ ] 假时钟+假Notifier测试：10月20日截止对应17/19/20日09:00；08:00截止配09:00提醒拒绝；无截止日期提醒拒绝。
- [ ] 测试睡眠错过多条后只提交一批；重复tick不再次提交；任务完成/删除/改期后旧revision提交次数=0。
- [ ] 测试时钟回拨无重复、时区改变重建pending、通知失败保留inbox并最多重试3次且间隔1/5/15分钟；崩溃后claimed超时60秒可重新处理。
- [ ] 运行`cargo test --manifest-path src-tauri/Cargo.toml --test reminders`确认失败。
- [ ] 实现持久化待发送、取消与补发、最近唤醒和30秒墙上时钟复核；恢复事件不可用时由轮询覆盖；加入reminders_changed事件。
- [ ] 测试通过并在已安装应用中检查隐藏窗口与睡眠恢复后60秒内提交，提交`feat: schedule persistent local reminders`。

## Task 6: 用户界面与完整操作闭环

**Files:** 创建src/app下五个组件、features/tasks/{api.ts,types.ts,TaskList.test.tsx,TaskEditor.test.tsx}、recurrence/{RecurrenceEditor.tsx,RecurrenceEditor.test.tsx}、reminders/{ReminderEditor.tsx,ReminderInbox.tsx}、styles/theme.css。

**Interfaces:** 前端types匹配Rust序列化DTO；api.ts提供createTask/updateTask/listTasks/setCompleted/setDeleted/saveSeries/editSeriesFrom和事件订阅封装。视图日期边界由Rust决定，前端仅格式化。

- [ ] 测试空标题不能提交、服务器保存失败保留输入并显示错误、完成失败恢复勾选；不同视图同一任务共享id与完成状态。
- [ ] 测试重复任务修改必须选择本次/本次及以后；选择后发送对应命令；提醒表单展示日期与时刻预览。
- [ ] 运行`npm test -- --run`确认关键用例失败。
- [ ] 实现中文侧栏、任务列表与编辑、重复/提醒设置、主题、搜索、优先级、键盘操作、单层子任务清单及父任务完成确认。
- [ ] 运行`npm test -- --run`及`npm run build`通过；手工检查100%/150%缩放、键盘焦点和长中文换行，提交`feat: add todo desktop workflows`。

## Task 7: 数据恢复与设置

**Files:** 创建storage/backup.rs、tests/backup.rs；修改Settings.tsx、commands.rs、repository.rs、lifecycle.rs。

**Interfaces:** export_archive(path: &Path) -> AppResult<()>；restore_archive(path: &Path) -> AppResult<RestoreReport>；backup_sqlite(path: &Path) -> AppResult<()>；update_settings(SettingsPatch) -> AppResult<Settings>。RestoreReport含恢复任务/模板数量。

- [ ] 测试往返导出保留任务、模板、实例游标及子项；损坏/未知版本导入数据不变；迁移失败当前库可恢复；恢复不重发历史已提交提醒。
- [ ] 运行`cargo test --manifest-path src-tauri/Cargo.toml --test backup`确认失败。
- [ ] 实现一致快照、版本化JSON、校验后替换与恢复前备份，暂停调度及重建pending；迁移失败禁止写入。
- [ ] 实现主题/时区/09:00默认时刻/登录启动设置；登录启动默认关闭，后台实际结果回写界面。
- [ ] 测试通过并手工恢复一份含重复模板的导出，提交`feat: support safe backup and desktop settings`。

## Task 8: 发布与系统验收

**Files:** 完成docs/{acceptance.md,release.md}、tauri.conf.json、发布脚本与CI配置。

**Interfaces:** 交付NSIS安装包、校验和、已知限制、数据位置和恢复指南；无需发布到外部服务。

- [ ] 运行`cargo fmt --manifest-path src-tauri/Cargo.toml --check`、`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`、`cargo test --manifest-path src-tauri/Cargo.toml`、`npm test -- --run`、`npm run build`，全部通过。
- [ ] 运行`npm run tauri build`；在干净Windows 10/11 x64环境检查安装、WebView2、离线使用、通知、托盘、登录启动和重复启动。
- [ ] 检查覆盖升级后数据保留、损坏备份导入、睡眠恢复、跨日视图和系列编辑；填入产品设计9项验收结果。
- [ ] 记录测试机配置并测10,000任务冷启动与查询p95，确认是否满足≤3秒/≤200毫秒目标；处理不满足项后确定发布门槛。
- [ ] 提交`release: package verified local todo desktop app`；交付安装包与验收报告。若用户要求外部发布，再进行相应发布操作。

## 实现前的决策与覆盖检查

已覆盖任务增删改、日期视图、重复生成、提醒、托盘、设置、备份和安装验收。用户八项选择均已纳入；实现前落实新工程绝对路径。当前请求仅为开发方案，以上步骤尚未执行，工期是估算。
