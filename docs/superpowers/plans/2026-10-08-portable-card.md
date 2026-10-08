# Portable Todo and Desktop Card Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Windows便携程序，程序旁data目录存储全部应用数据，增加可置顶桌面小卡片。

**Architecture:** 同一Rust进程、SQLite服务和调度线程管理main/card两个窗口。卡片复用任务API和变更事件；便携路径由current_exe解析，与启动工作目录无关。发布为exe+ZIP，不调用安装器。

**Tech Stack:** 已有Tauri2/Rust/React/SQLite，无新增运行依赖。

**Spec:** 用户本轮三项要求及已确认原方案。

## Global Constraints

- 使用便携程序，不启动安装版；旧安装数据保留，不自动覆盖或删除。
- 数据库、卡片偏好及WebView2缓存放在exe同目录data文件夹；不可写时明确启动失败，禁止静默回退AppData。
- 主界面与卡片共享任务，完成/新建后同步；卡片关闭仅隐藏，后台提醒继续。
- 置顶默认开启，可关闭并持久化；卡片有拖动区域，可调整大小。

## Review Focus

- 从其他工作目录启动仍使用exe/data；目录不可写不得错误写到其他位置。
- 卡片完成失败保留状态并提示；重复点击避免重复写入。
- 快速添加空标题拒绝、失败保留输入、成功后两窗口同步。
- 置顶失败不得假显示成功；关闭卡片后主界面/托盘能重新打开。
- 发布ZIP含程序且没有开发数据库、测试任务和安装器。

### Task 1: Portable runtime and packaging

**Files:** src-tauri/src/{portable.rs,desktop.rs,commands.rs,main.rs}、tauri.conf.json、scripts/build.ps1、.gitignore。
**Interfaces:** data_directory(exe:&Path)->Result<PathBuf>; AppState.data_directory供备份路径保护使用。
- [x] 路径测试固定exe父目录、拒绝无父目录的输入。
- [x] 实现exe/data与所有WebView缓存路径、改为no-bundle发布并复制到portable/LocalTodo。
- [x] 核心测试、Clippy、生产构建通过。

### Task 2: Desktop card

**Files:** src/app/DesktopCard.tsx、DesktopCard.test.tsx、src/styles/card.css、main.tsx、App.tsx、api.ts、desktop.rs、commands.rs、capabilities/default.json。
**Interfaces:** open_card/hide_card/card_pin/set_card_pin/open_main commands；今日快速新增使用save_task，完成使用set_completed，订阅tasks_changed。
- [x] 组件测试快速添加、保存失败保留、完成失败、置顶真实结果；先失败再实现。
- [x] 实现卡片窗口、拖动、置顶持久化、主界面/托盘入口与同步。
- [x] 原生烟雾测试主界面入口、卡片添加/完成与主界面同步、置顶状态、隐藏重开。

### Task 3: Delivery

**Files:** README.md、docs/portable-release.md、scripts/portable-smoke.ps1。
- [x] 构建便携exe/ZIP，检查无测试数据。
- [x] 停止本次启动的旧安装实例，运行便携版验证exe/data与Windows窗口行为。
- [x] 记录验证结果与真实系统限制；提交并交付便携程序和ZIP。
