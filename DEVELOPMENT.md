# 开发与发布

## 环境与结构

面向 Windows 10/11 x64 开发。需要 Node.js 24、Rust（版本以 `rust-toolchain.toml` 为准）、MSVC C++ 构建工具、Windows SDK 和 WebView2 Runtime。

- `src/`：React/TypeScript 界面，主窗口和桌面小卡片共用任务服务。
- `src-tauri/`：Rust 桌面宿主、托盘、单实例、系统通知、登录启动和文件对话框。
- `crates/todo-core/`：日期与重复规则、SQLite 持久化、提醒调度和备份恢复。
- `scripts/build.ps1`：唯一的项目辅助脚本，负责构建及便携打包。
- `.github/workflows/ci.yml`：Windows 自动构建和版本标签发布。

## 一键构建

从项目根目录运行：

```powershell
./scripts/build.ps1
```

脚本优先使用工程内 `.tools/cargo`、`.tools/rustup`，否则使用系统工具链。缺少前端依赖时自动执行 `npm ci`；随后通过 Tauri 执行一次前端生产构建和一次 Rust release 构建，使用 `--locked` 锁定 Cargo 依赖，不运行测试或生成安装器。

输出：

- `portable/LocalTodo/local-todo.exe`
- `portable/LocalTodo/Open-Desktop-Card.cmd`
- `portable/LocalTodo-版本号-windows-x64.zip`
- 同名 `.zip.sha256` 校验文件

ZIP 从独立暂存目录打包，只包含程序、卡片入口、说明及空 `data` 目录，不包含已有用户数据。重新构建保留 `portable/LocalTodo/data`。构建前先从托盘退出正在运行的便携实例，以便替换 EXE。

可指定输出目录，或仅打包已有 release EXE：

```powershell
./scripts/build.ps1 -OutputDirectory .tools/build-output
./scripts/build.ps1 -SkipBuild -OutputDirectory .tools/package-preview
```

更改 npm 依赖或锁文件后，运行 `npm ci --no-audit --no-fund` 更新本地依赖。PowerShell 执行策略阻止脚本时，可使用 `pwsh -NoProfile -ExecutionPolicy Bypass -File ./scripts/build.ps1`。

## 本地开发

使用系统 Node/Rust 工具链时：

```powershell
npm ci --no-audit --no-fund
npm run tauri dev
```

单独查看前端可运行 `npm run dev`；任务操作依赖 Tauri/Rust 宿主。需要工程内 Rust 工具链时，在当前 PowerShell 会话设置：

```powershell
$env:CARGO_HOME = Join-Path $PWD '.tools/cargo'
$env:RUSTUP_HOME = Join-Path $PWD '.tools/rustup'
$env:PATH = "$env:CARGO_HOME/bin;$env:PATH"
```

本地按需检查可直接运行 `npm run build`、`npm test`、`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings` 或 `cargo test --workspace`。构建脚本和 GitHub Actions 仅运行生产构建，不执行测试、fmt 或 Clippy。

调试构建可通过环境变量 `LOCALTODO_TEST_DATA_DIR` 指向隔离数据目录；正式构建忽略此覆盖，始终将任务、卡片偏好及 WebView2 缓存放在 EXE 旁的 `data/`。窗口创建使用 `WebviewWindowBuilder.data_directory` 显式设置缓存目录。

## GitHub Actions

- 推送 `main`/`master`、面向这些分支的 PR，以及手动运行：构建并上传便携产物。
- 推送 `v*` 版本标签：构建并发布 GitHub Release，附件为 ZIP 和 SHA-256。
- 仅文档修改跳过自动构建。主分支/PR产物保留14天；Release 附件长期保留。
- 通过内置 `GITHUB_TOKEN` 发布，无需另配 Token；仓库或组织策略需允许 `contents: write`。
- 工作流需推送到 GitHub 才能执行；手动入口要求工作流已存在于默认分支。手动选择已有版本标签可重新构建并更新同名附件。

流程缓存 npm 下载、`node_modules` 和 Rust 依赖及 `target/` 编译产物。`node_modules` 缓存命中时跳过 `npm ci`；Rust 工具链只安装最小组件。ZIP 上传不再次压缩，同分支的新提交会取消旧构建，版本标签构建不会被取消。首次构建仍需下载和编译依赖，后续耗时取决于缓存命中和修改范围。

## 发布新版本

同步修改 `src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`package.json` 中的版本，更新 `Cargo.lock` 中的应用版本，并运行以下命令更新 npm 锁文件：

```powershell
npm install --package-lock-only --ignore-scripts --no-audit --no-fund
```

提交版本修改后，推送与应用版本一致的标签。例如当前版本为 `0.2.0`：

```powershell
git tag v0.2.0
git push origin v0.2.0
```

版本不一致时构建脚本会报错，标签与应用版本不一致时工作流会提前终止。含 `-` 的版本标签发布为预发布版。已有 Release 再次运行会覆盖同名附件。
