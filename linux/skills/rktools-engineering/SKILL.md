---
name: rktools-engineering
description: 按固定 Cargo workspace、完整后端分层、端口适配器解耦、类型化烧录操作、安全设备状态语义和单一出包入口重构、扩展、评审或修复 rktools Rust/egui 工程，并清理已确认重复、冗余、无效或不可达的代码。适用于本仓库的配置模型、应用用例、Rockchip upgrade_tool 适配、进程运行时、桌面 UI、构建与 Debian 离线交付改动。
---

# RKTools 工程模式

在不削弱分层、设备安全和交付边界的前提下修改 rktools。复用现有 Rust/egui 技术栈和外部 `upgrade_tool` 适配，不把闭源二进制包装描述为开源协议实现。

## 从实际生效代码开始

- 设计改动前，检查相关源码、测试、`docs/`、`packaging/`、`scripts/` 和当前 Git 差异。
- 将已有未提交内容视为用户资产；只修改本次需求涉及的文件，不覆盖、清理或格式化无关改动。
- 通过首个真实错误、实际调用关系、测试或真实 UI 预览定位根因，不新增临时兼容分支掩盖症状。
- 先确认当前 `Cargo.toml`、`rust-toolchain.toml` 和脚本入口，不复用旧文件名或旧包名。

## 保持依赖方向

本仓库固定为五 crate workspace，不再在仓库根目录建立 `src/`：

```text
rktools-launcher -> rktools-frontend -> rktools-api -> rktools-config
        |                                      ^
        +--------> rktools-backend -------------+
                          +------------------------> rktools-config
```

- `app/api/` 的 Cargo package 为 `rktools-api`，是前后端唯一共享契约，只保存能力接口、类型化操作、设备状态和执行事件；禁止依赖 backend、frontend、文件系统、进程、平台与 UI。
- `app/config/` 的 Cargo package 为 `rktools-config`，只包含镜像/工作区配置模型、业务校验和 JSON codec；禁止文件系统、进程、设备、平台与 UI 依赖。
- `app/backend/` 的 Cargo package 为 `rktools-backend`，必须保持 `domain -> application ports <- infrastructure adapters` 的依赖反转；禁止依赖 frontend。
- `app/backend/src/domain/` 只保存后端内部执行计划；跨前后端的设备、操作和执行事件统一来自 `rktools-api`。禁止文件系统、进程、GUI、厂商适配器和应用门面依赖。
- `app/backend/src/application/` 不直接执行命令或读写文件；`facade.rs` 的 `FlashApplication` 是前后端唯一门面，纯匹配和校验规则分别放在职责明确的用例模块。
- `app/backend/src/application/ports/` 必须按职责隔离端口：任务执行、设备监控、工具定位、操作日志、规划/协议、工作区仓储和最小文件系统查询不得重新合并成万能 Runtime 或 Repository 接口。
- `app/backend/src/infrastructure/process/` 是创建外部进程的唯一位置，并按命令环境、只读轮询、日志映射、终端清洗和任务协调拆分；通用进程模块禁止出现 Rockchip、`upgrade_tool` 和厂商命令码。
- `app/backend/src/infrastructure/rockchip/` 集中保存工具发现、Rockchip profile、命令映射和输出解析；厂商特例不得扩散到应用或前端。
- `app/backend/src/infrastructure/persistence/` 分开实现工作区仓储和本地文件系统端口；JSON 编解码复用 config crate，不维护第二份模型。
- `app/frontend/` 的 Cargo package 为 `rktools-frontend`，只依赖 `rktools-api`，管理 egui 桌面状态、输入和反馈，并提交类型化操作；固件元数据、工作区、parameter 和设备访问必须经 `FlashService`，禁止依赖 backend、启动进程或直接依赖后端内部适配器。界面专用字体发现隔离在 `desktop/platform/`。
- `app/launcher/` 的 Cargo package 和二进制名为 `rktools`，只连接 `rktools_backend::create_application` 与 `rktools_frontend::desktop::run`，不放业务、设备、配置或 UI 实现。

全部 Rust 源码统一放在 `app/`；其下目录只写职责名：`api`、`backend`、`config`、`frontend`、`launcher`。Cargo package 使用 `rktools-*` 前缀。Rust 模块和源码使用 `snake_case`，外部二进制、desktop、udev 和安装路径继续遵循平台契约。

新增抽象必须确实隔离变化或依赖；小功能放入职责最接近的现有模块，不为目录对称增加空层、`utils.rs`、兼容转发或第二套门面。

审计“分离分层、全后端解耦”时必须检查实际 import、端口职责和系统调用位置，不能仅凭 crate 或目录名称判定完成。禁止用 `#[allow]`、公开基础设施类型或前端包装系统调用掩盖跨层依赖。

## 类型化操作与唯一命令映射

- GUI、确认框、校验和后端规划共享同一个 `FlashOperation` / 领域枚举契约，不跨层复制命令字符串或状态判断。
- `app/backend/src/infrastructure/rockchip/upgrade_tool.rs` 是 Rockchip 命令映射的单一权威位置；通用进程运行时不感知 `DI`、`WL`、`UF`、`RID` 等命令码。
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

- `rktools-backend` 用 `FlashApplication` 实现 `FlashService::import_parameter`，负责协调读取、解析、目录枚举和结果返回；实际文件系统访问继续经 `ConfigurationRepository`。
- 导入 `parameter.txt` 时保留 `parameter` 项，并在同目录不区分大小写匹配同名 `.img`、`.bin`、`.txt` 或无扩展名固件；`rootfs:grow` 等后缀不属于文件名。
- 命中的项填入路径并启用；缺失项清空路径、保持未启用并向用户明确报告，禁止猜测跨目录文件。
- 标准分区继续由 Rockchip profile 映射；未知分区按已解析起始扇区走通用写入，不在 UI 复制分区表。

## 构建不会失真的 egui 界面

- 复用 `app/frontend/src/desktop/theme.rs`、现有组件、导航和视觉层级，避免在功能代码中散落同义视觉常量。
- 响应式变化由内容与控件实际所需宽度决定；窄宽度时采用堆叠或滚动，不强塞横排。
- 滚动区要为固定操作区预留空间；输入框宽度扣除按钮和间距，卡片需要占满时显式使用可用宽度。
- `egui::Area` 或浮层内谨慎使用会占满剩余空间的布局；右对齐按钮优先放进内容自适应的水平行，避免提示框铺满窗口。
- 视觉改动必须运行真实原生程序，至少检查默认窗口、最小窗口和一个布局切换附近宽度；确认无重叠、裁剪、不可达按钮、错误滚动或状态误导。UI 预览不得触发真实写入、擦除或升级。

## 保持出包分层与干净产物

- 用户入口只有根目录 `./build.sh`；入口只负责参数分发和装配，开发生命周期与交付生命周期由子命令区分。
- `scripts/lib/rust.bash` 保存 setup、toolchain、release、run、fmt、test、lint、architecture、check 和 clean 的 Rust 工作区流程；公开 `build` 默认进入 `.deb` 交付流程，`package` 是其别名。`scripts/docker.bash` 仅作为宿主组合根，交付用例放在 `scripts/lib/delivery.bash`，Docker 机制放在 `scripts/lib/container.bash`。Rust、rustfmt、Clippy、系统开发库和打包工具都必须预装在镜像中；除用户显式执行 `install` 外，禁止构建流程调用宿主机的 sudo、apt、cargo 或 rustup。
- `scripts/lib/` 只保留通用命令、Rust/ELF 检查与 Debian/离线依赖流程；不得出现 rktools、Rockchip、`upgrade_tool`、图标或 udev 特例。
- `scripts/config/rktools.bash` 保存项目名、文件布局、工具校验、架构门禁和打包钩子；`scripts/config/rktools-delivery.bash` 保存发行版矩阵、镜像覆盖规则、环境变量映射和安装验收清单。项目差异通过 `project_*` 钩子注入，不在通用层增加项目名、固定版本或安装路径分支。
- 保持 `packaging/runtime-dependencies.txt` 为运行依赖单一来源，并校验被打包 `upgrade_tool` 的固定摘要、amd64 架构和二进制兼容性。
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
./build.sh fmt
./build.sh test
./build.sh lint
./build.sh release
git diff --check
```

同时用 `./build.sh architecture` 确认命令码、进程和项目特例没有跨层泄漏。脚本改动补充 `bash -n`；视觉改动补充真实 GUI 尺寸复核；打包改动在用户要求出包时运行 `./build.sh build`，需要离线交付时再运行 `./build.sh offline`。

静态审计、单元测试、Clippy、release 构建、GUI 预览、Debian 出包、离线依赖验证和真实 Rockchip 硬件烧录是不同验证层级，必须分别说明。除非用户明确要求，否则不提交、不推送，不执行真实设备写入、擦除、升级或模式切换。
