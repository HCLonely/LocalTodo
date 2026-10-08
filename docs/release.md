> 此文为0.1.0历史安装版记录。当前交付已改为0.2.0便携版，见[portable-release.md](portable-release.md)。

# 首版本地交付

日期：2026-10-08；版本：0.1.0；平台：Windows x64；名称：拾序 · LocalTodo。

## 安装与启动

安装包：`D:\GithubProgram\AI\TodoList\target\release\bundle\nsis\LocalTodo_0.1.0_x64-setup.exe`。

大小：3,265,244字节，约3.11MiB。

SHA-256：

```
fd56902622a0508923dc3539d47f57587cd3e56e89e402360c379f0b11ba21b7
```

安装包旁的`.exe.sha256`保存同一校验值，build.ps1后续构建会自动更新。

本机已安装至`%LOCALAPPDATA%\LocalTodo\local-todo.exe`，开始菜单名称LocalTodo。默认不设置登录启动；关闭窗口继续驻留托盘，右键托盘可退出。

## 本次内容

本日/周/月/指定日期、安排到今天、优先级、搜索、子任务、每天/周/月重复、多个提前天数提醒及日期预览、提醒中心、浅深主题、时区、可选登录启动、回收站、JSON导出恢复与SQLite快照。全部数据在本机，无账号或云同步。

首次安装若缺少WebView2，安装器需联网下载运行时；日常操作不需要网络。

## 验证

- Rust20项、前端11项全部通过；fmt、全目标Clippy、TypeScript与生产构建通过。
- 真实WebView2/Rust/SQLite操作9项通过，包含原生对话框取消/确认。
- 静默覆盖安装退出0，本机原空库与设置指纹保持；已安装exe与release仅相差Tauri定义的NSIS标记，内容核对通过。
- 安装版关闭驻留、重复启动激活已有窗口、Windows通知提交通过。本次启动到UI可操作866ms，不能视为干净系统或万任务冷启动结果。
- 10,000任务核心查询20次，median/p95 15ms；不等于UI启动指标。

结果文件：desktop-smoke-results.json、installed-smoke-results.json、release-verification.json。详细审查处理和实施决定见implementation-record.md。

## 后续系统验收

干净Windows10/11、真实睡眠恢复、通知关闭/勿扰、注销后的登录启动、大量真实数据跨版本升级及不同系统缩放仍需独立验证。安装包未配置代码签名。本次仅本地交付，尚未公开发布或推送远程。

任务数据位置、备份恢复与开发命令见项目README；完整验收状态见acceptance.md。
