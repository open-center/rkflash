---
name: rkflash-engineering
description: 按固定 Cargo workspace、完整后端分层、端口适配器解耦、类型化烧录操作、安全设备状态语义和单一出包入口重构、扩展、评审或修复 rkflash Rust/egui 工程，并清理已确认重复、冗余、无效或不可达的代码。适用于本仓库的配置模型、应用用例、Rockchip upgrade_tool 适配、进程运行时、桌面 UI、构建与 Debian 离线交付改动。
---

# RKFlash 工程模式

在不削弱分层、设备安全和交付边界的前提下修改 rkflash。复用现有 Rust/egui 技术栈和外部 `upgrade_tool` 适配，不把闭源二进制包装描述为开源协议实现。

## 从实际生效代码开始

- 设计改动前，检查相关源码、测试、`docs/`、`platforms/linux/packaging/files/`、`scripts/` 和当前 Git 差异。
- 将已有未提交内容视为用户资产；只修改本次需求涉及的文件，不覆盖、清理或格式化无关改动。
- 通过首个真实错误、实际调用关系、测试或真实 UI 预览定位根因，不新增临时兼容分支掩盖症状。
- 先确认当前 `Cargo.toml`、`rust-toolchain.toml` 和脚本入口，不复用旧文件名或旧包名。

## 保持依赖方向

Rust 工作区位于仓库根目录，按“通用分层 + 平台拆包”组织（完整说明见 `docs/分层架构.md`）：

```text
launcher → platforms/{linux,windows} → runtime → application → core
frontend → api        application → api, config
```

- `crates/core/`（`rkflash-core`）：领域核心——固件解析、分区表、芯片识别、工具输出识别、rkdeveloptool 协议与烧录工作流；不依赖任何其他 crate、界面或进程管理。
- `crates/api/`、`crates/config/`：前后端唯一契约与工作区配置模型；不依赖任何实现或系统能力。
- `crates/api/` 按前端功能区分模块（device、task、download、upgrade、advanced、settings、log、workspace），每个模块一个接口 trait；改变设备的操作成对提供 `check_*` 与执行函数。
- `crates/application/`：`service/<模块>.rs` 实现同名 api 接口，内部统一转为 `domain::FlashOperation` 再交给平台规划；另含操作校验、镜像路径解析、执行计划（`domain/`）与端口（`ports/`）；端口按职责隔离（任务执行、设备监控、工具定位、操作日志、规划/协议、工作区仓储、文件系统），不得合并成万能接口；不访问文件系统或进程。
- `crates/runtime/`：创建外部进程的唯一位置（执行、会话、轮询、日志、终端清洗、持久化、工具发现、装配）；平台差异只经 `OsHost` 注入，禁止 `cfg(windows/unix)`、系统专有 API 和任何烧录工具/厂商细节。
- `platforms/<系统>/` 放该系统的全部差异：`platforms/linux/`（upgrade_tool 适配器、Unix 宿主、`desktop.rs` 桌面配置、`packaging/`）、`platforms/windows/`（rkdeveloptool 适配器、Win32 宿主、`desktop.rs`、`packaging/`）、`platforms/macos/`（SwiftUI 应用）。两个 Rust 平台包互不依赖；适配器与宿主不得依赖界面，只有 `desktop.rs` 依赖 `rkflash-frontend`。`third_party/` 存放 Windows 与 macOS 共用的 rkdeveloptool/libusb 源码。
- `crates/frontend/`：只依赖 `rkflash-api`，不含平台条件编译、不按工具名分支（用 `ToolInfo`）；视图（`pages/`、`chrome.rs`、`overlays.rs`）只调用动作层，`desktop/actions/<模块>.rs` 是唯一调用 api 的地方且只调用同名 api 模块；字体、渲染器、启动失败提示由平台的 `DesktopProfile` 注入。
- `crates/launcher/`：只按目标平台选择平台包，调用 `rkflash_frontend::desktop::run(create_application(), desktop_profile())`。

新增功能时按“api 模块 → application `service/` 同名模块 → frontend `actions/` 同名模块 → 页面”逐层添加。新增平台时新建 `platforms/<名称>`，实现 `FlashPlanner`（含 `tool_info()`）、`ToolProtocol`、`OsHost` 与 `desktop_profile()`，用 `runtime::assemble_application` 装配，不修改通用层。Cargo package 使用 `rkflash-*` 前缀，Rust 模块使用 `snake_case`。

新增抽象必须确实隔离变化或依赖；小功能放入职责最接近的现有模块，不为目录对称增加空层、`utils.rs`、兼容转发或第二套门面。

审计“分离分层、全后端解耦”时必须检查实际 import、端口职责和系统调用位置，不能仅凭 crate 或目录名称判定完成。禁止用 `#[allow]`、公开基础设施类型或前端包装系统调用掩盖跨层依赖。

## 类型化操作与唯一命令映射

- GUI、确认框、校验共享 api 中的请求类型与枚举，后端规划统一使用应用层的 `FlashOperation`；不跨层复制命令字符串或状态判断。
- `platforms/linux/src/upgrade_tool.rs` 是 Rockchip 命令映射的单一权威位置；通用进程运行时不感知 `DI`、`WL`、`UF`、`RID` 等命令码。
- 外部工具必须使用独立 program/argv 调用，禁止拼接用户可控 Shell 命令。
- 写入、擦除、升级 Loader、切换存储等改变设备或数据的操作保留明确校验与确认；无设备或多设备时拒绝执行。
- 结果同时分析退出码和已清洗的 stdout/stderr 失败文本；先统一去除 ANSI/VT100 控制序列，再做进度和失败解析。
- 工具进程仍以工具目录为 `current_dir` 读取同目录配置；运行数据和日志写入前端可配置的用户级目录，不能默认写安装目录。

## 保守处理设备模式

- 设备扫描和查询是只读动作，不得隐式注入 `DB <Loader>`、`RD 3` 或其他模式切换。
- 不恢复 `DownloadBoot`、`EnterMaskrom`、自动 Loader 选择、模式快捷按钮或查询前状态变更；显式“升级 Loader”仍是独立的用户操作。
- 软件动作仅面向已通过 Rockchip VID `2207` 枚举的设备。未枚举板卡的恢复依赖具体板型的 Recovery/Maskrom 按键或 test-point，不暗示 GUI 可以远程切换离线硬件。
- Maskrom、Loader 和“未发现设备”文案必须区分当前检测状态、可执行软件动作和现场板级恢复步骤。

## 保持 parameter 与固件匹配契约

- `rkflash-application / rkflash-runtime` 用 `FlashApplication` 实现 `DownloadApi::import_parameter`，负责协调读取、解析、目录枚举和结果返回；实际文件系统访问继续经 `ConfigurationRepository`。
- 导入 `parameter.txt` 时保留 `parameter` 项，并在同目录不区分大小写匹配同名 `.img`、`.bin`、`.txt` 或无扩展名固件；`rootfs:grow` 等后缀不属于文件名。
- 命中的项填入路径并启用；缺失项清空路径、保持未启用并向用户明确报告，禁止猜测跨目录文件。
- 标准分区继续由 Rockchip profile 映射；未知分区按已解析起始扇区走通用写入，不在 UI 复制分区表。

## 构建不会失真的 egui 界面

- 复用 `crates/frontend/src/desktop/theme.rs`、现有组件、导航和视觉层级，避免在功能代码中散落同义视觉常量。
- 响应式变化由内容与控件实际所需宽度决定；窄宽度时采用堆叠或滚动，不强塞横排。
- 滚动区要为固定操作区预留空间；输入框宽度扣除按钮和间距，卡片需要占满时显式使用可用宽度。
- `egui::Area` 或浮层内谨慎使用会占满剩余空间的布局；右对齐按钮优先放进内容自适应的水平行，避免提示框铺满窗口。
- 视觉改动必须运行真实原生程序，至少检查默认窗口、最小窗口和一个布局切换附近宽度；确认无重叠、裁剪、不可达按钮、错误滚动或状态误导。UI 预览不得触发真实写入、擦除或升级。

## 保持出包分层与干净产物

- 用户入口只有根目录 `platforms/linux/packaging/build.sh`；入口只负责参数分发和装配，开发生命周期与交付生命周期由子命令区分。
- `platforms/linux/packaging/scripts/lib/rust.bash` 保存 setup、toolchain、release、run、fmt、test、lint、architecture、check 和 clean 的 Rust 工作区流程；公开 `build` 默认进入 `.deb` 交付流程，`package` 是其别名。`platforms/linux/packaging/scripts/docker.bash` 仅作为宿主组合根，交付用例放在 `platforms/linux/packaging/scripts/lib/delivery.bash`，Docker 机制放在 `platforms/linux/packaging/scripts/lib/container.bash`。Rust、rustfmt、Clippy、系统开发库和打包工具都必须预装在镜像中；除用户显式执行 `install` 外，禁止构建流程调用宿主机的 sudo、apt、cargo 或 rustup。
- `platforms/linux/packaging/scripts/lib/` 只保留通用命令、Rust/ELF 检查与 Debian/离线依赖流程；不得出现 rkflash、Rockchip、`upgrade_tool`、图标或 udev 特例。
- `platforms/linux/packaging/scripts/config/rkflash.bash` 保存项目名、文件布局、工具校验、架构门禁和打包钩子；`platforms/linux/packaging/scripts/config/rkflash-delivery.bash` 保存发行版矩阵、镜像覆盖规则、环境变量映射和安装验收清单。项目差异通过 `project_*` 钩子注入，不在通用层增加项目名、固定版本或安装路径分支。
- 保持 `platforms/linux/packaging/files/runtime-dependencies.txt` 为运行依赖单一来源，并校验被打包 `upgrade_tool` 的固定摘要、amd64 架构和二进制兼容性。
- 正式 `.deb` 只能在 Ubuntu 22.04 基线容器中编译并复用于 22.04/24.04/26.04；离线依赖必须按目标发行版分别解析，禁止在较新 Ubuntu 中重新编译应用后宣称向下兼容。
- `offline`/`offline-all` 必须在对应版本的干净基础容器中使用 `--network none` 和 `apt-get --no-download` 实装验收，不能只验证下载数量或校验和。
- 本机和离线安装都必须先把 `.deb` 暂存到 `/var/tmp` 下对 `_apt` 可遍历的目录；禁止通过放宽用户家目录权限或设置 `APT::Sandbox::User=root` 绕过 APT 沙盒。
- Cargo 编译缓存和程序必须位于 `output/target/`；`package` 和 `offline` 的交付物位于 `output/dist/`，生成前会安全重建该目录。缺失构建依赖时应更新并重建 Docker 镜像，不得临时修改宿主软件。

## 清理本次涉及范围的无效逻辑

- 完成实现后，对涉及模块做调用关系审计；删除已经确认无调用、不可达、被新流程取代或只转发旧内部路径的函数、枚举项、字段、分支和导出。
- 同一行为只保留一个权威入口；UI、应用服务和适配器不得各自维护默认值、路径匹配、命令映射或设备状态策略。
- 删除前用 `rg`、编译器、测试或真实调用证明；名称相似但安全、持久化或设备语义不同的代码不得强行合并。
- 改名或移动后同步模块声明、测试、文档、desktop、打包路径和安装契约，并检查 `include_*`、相对路径与环境变量引用。
- 不用新环境特判、空壳兼容层、重复 fallback、Clippy 忽略或测试放宽替代根因修复。

## 按风险验证并诚实交付

普通源码改动至少运行：

```bash
platforms/linux/packaging/build.sh fmt
platforms/linux/packaging/build.sh test
platforms/linux/packaging/build.sh lint
platforms/linux/packaging/build.sh release
git diff --check
```

同时用 `platforms/linux/packaging/build.sh architecture` 确认命令码、进程和项目特例没有跨层泄漏。脚本改动补充 `bash -n`；视觉改动补充真实 GUI 尺寸复核；打包改动在用户要求出包时运行 `platforms/linux/packaging/build.sh build`，需要离线交付时再运行 `platforms/linux/packaging/build.sh offline`。

静态审计、单元测试、Clippy、release 构建、GUI 预览、Debian 出包、离线依赖验证和真实 Rockchip 硬件烧录是不同验证层级，必须分别说明。除非用户明确要求，否则不提交、不推送，不执行真实设备写入、擦除、升级或模式切换。
