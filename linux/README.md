# RKFlash（Linux）

rkflash 是一个面向 Rockchip 开发板的纯 Rust 桌面烧录前端。界面使用
`egui/eframe`，后台以参数数组直接调用 Rockchip `upgrade_tool`，不经过 shell，支持设备轮询、
实时日志、任务进度和中止操作。

> 当前版本是可运行的第一阶段：Windows RKDevTool 的三个主工作区已经建立，常用命令已接通。
> `upgrade_tool` 是 Rockchip 的闭源二进制，本项目不会将它当成开源代码。源码仓库默认忽略该文件；
> 本地构建安装包时会将用户准备并通过校验的二进制装入 `.deb`。

## 文档导航

- [代码总览与文档索引](docs/代码总览.md)：仓库结构、crate 依赖方向、设计原则速查。
- [契约层详解](docs/契约层.md)：`rkflash-api` / `rkflash-config` 的类型与 `FlashService` 端口。
- [后端详解](docs/后端.md)：domain / application / infrastructure 逐模块与命令码全表。
- [前端详解](docs/前端.md)：帧循环、轮询协议、页面与组件、主题与中文字体。
- [构建脚本体系详解](docs/构建脚本.md)：build.sh、Dockerfile 与 lib/config 分层、离线包流程。
- [打包与交付详解](docs/打包与交付.md)：udev 权限、`upgrade_tool` 生命周期、`.deb` 与离线包安装。
- [分层架构说明](docs/分层架构.md)、[功能矩阵](docs/功能矩阵.md)、
  [Linux 兼容性说明](docs/兼容性说明.md)。

## 已实现

- 下载镜像
  - parameter、uboot、trust、misc、resource、kernel、boot、recovery、system
  - 未知分区按起始扇区使用 `WL` 写入
  - 加载 Rockchip `parameter.txt` 后展开分区定义，并按分区名自动匹配同目录的 `.img` / `.bin` / `.txt` 固件
  - 下载项增删、排序、启用和 JSON 配置导入/导出
  - 可选“完成后重启”
- 升级固件
  - 使用 `UF` 升级 `update.img`
  - `-noreset` 选项
  - 使用固件参数执行 `EF` 擦除
- 高级功能
  - 读取 Flash ID、Flash 信息、Chip 信息、Capability、分区表、USB 枚举序列号和安全模式
  - 进入 Maskrom 或 Loader 模式时弹出只读状态说明；程序不主动加载 Loader，也不主动进入 Maskrom
  - 测试、重启、切换存储、清空 VendorStorage SN 和重置 USB 管道
  - 升级 Loader 并选择存储类型
  - LBA 读取、写入和擦除
  - 串口日志导出
- 运行安全
  - 写入、擦除和状态切换前确认
  - 同一时刻只允许一个任务
  - 多设备时拒绝执行，避免误刷
  - 子进程参数不经过 shell
  - 可中止运行中的 `upgrade_tool`
  - `upgrade_tool` 内部日志默认隔离到 `/tmp/rkflash/log/`，可在“高级功能”中直接修改日志目录
  - 每次操作开始前创建一份以本地时间戳命名的独立日志（如 `20260916-183045-127.log`）；烧录失败时弹出错误气泡，完整错误仍保留在运行终端和对应日志中

完整的功能对照和边界见 [功能矩阵](docs/功能矩阵.md)。

## 系统兼容性

同一个 `.deb` 支持 Ubuntu 22.04、24.04 和 26.04 amd64。正式发布程序固定在 Ubuntu
22.04 容器中编译，以 glibc 2.35 为最低兼容基线；构建安装包时会同时检查主程序和
随包提供的 `upgrade_tool` 的架构与 glibc 兼容性。
详细兼容边界与检查方法见 [Linux 兼容性说明](docs/兼容性说明.md)。

## 准备 `upgrade_tool`

从 Rockchip Linux SDK 或设备厂商提供的软件包中取得与你的芯片兼容的版本。程序按以下顺序查找：

1. 环境变量 `RK_UPGRADE_TOOL`；
2. 程序同目录的 `tools/upgrade_tool`；
3. 当前工作目录的 `tools/upgrade_tool`；
4. 系统安装路径 `/usr/lib/rkflash/upgrade_tool`；
5. `PATH`；
6. 界面右上角手动选择。

例如：

```bash
mkdir -p tools
cp /path/to/upgrade_tool tools/upgrade_tool
chmod +x tools/upgrade_tool
```

## Linux USB 权限

开发运行时可以安装仓库中的 udev 规则：

```bash
sudo cp packaging/71-rkflash.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add --subsystem-match=usb --attr-match=idVendor=2207
```

重新插拔设备后，以普通用户启动应用。除非用于临时诊断，不建议用 `sudo` 启动整个 GUI。

## Docker 构建

宿主机只需要安装并启动 Docker Engine，不需要安装 Rust、Clippy、编译库或 Debian
打包工具。拉取源码后直接执行：

```bash
./build.sh
```

`build.sh` 会用仓库根目录的 `Dockerfile` 构建镜像，然后在容器内执行实际命令。镜像默认以
Ubuntu 22.04 为基线，并包含固定版本的 Rust 工具链（版本由 `rust-toolchain.toml` 锁定，
项目最低要求 1.88）、rustfmt、Clippy、图形系统开发库、`dpkg-deb`、离线依赖解析工具和
其余编译组件；宿主 Ubuntu 的版本和已安装软件不会参与链接或打包。

无参数调用和 `./build.sh build` 默认都会编译 release 程序并生成 `.deb`。如果只需要 Rust
release 二进制，使用：

```bash
./build.sh release
```

开发调试时可以直接指定工具日志目录：

```bash
RKFLASH_LOG_DIR=output/logs ./build.sh run
```

Cargo 的编译缓存、中间文件和程序统一位于 `output/target/`，release 程序为
`output/target/release/rkflash`。Cargo 下载缓存位于 `output/docker/cargo-home/`。容器以当前
用户的 uid/gid 运行，因此产物不会变成只能由 root 修改的文件。

统一入口还提供完整的开发生命周期命令：

```bash
./build.sh              # 默认编译并生成 .deb
./build.sh build        # 同上
./build.sh release      # 只生成 Rust release 二进制
./build.sh setup        # 拉取基础镜像并准备完整构建环境
./build.sh toolchain    # 不使用缓存重建镜像（Rust 版本由 rust-toolchain.toml 锁定）
./build.sh check        # 格式、架构、测试和 Clippy
./build.sh clean        # 清理整个 output/
./build.sh help         # 查看全部命令
```

首次调用会创建 builder 镜像，后续命令直接复用；`setup` 会检查并更新镜像，`toolchain` 会
不使用缓存完整重建。修改 Dockerfile 后也可以设置 `RKFLASH_DOCKER_REBUILD=1` 强制重建。
`clean` 只清理仓库内的 `output/`，不启动 Docker，也不需要网络。默认目标平台是项目支持的
`linux/amd64`；需要覆盖时可设置
`RKFLASH_DOCKER_PLATFORM`。如果所在网络需要使用 Ubuntu 镜像站，可传入完整基础镜像名，
例如 `RKFLASH_UBUNTU_IMAGE=registry.example.com/library/ubuntu:22.04 ./build.sh build`；
`RKFLASH_UBUNTU_VERSION` 只选择开发环境或离线包的目标版本，不会改变正式 `.deb` 的
22.04 编译基线。APT 镜像可通过
`RKFLASH_UBUNTU_APT_MIRROR=https://mirror.example.com/ubuntu` 单独设置，该值也会自动用于
离线依赖包下载。企业 HTTPS 代理使用自定义根证书时，可再设置
`RKFLASH_DOCKER_CA_CERT=/path/to/company-ca.crt`（单张 PEM 证书），证书会安装到构建镜像中。
如果 APT 镜像和外部 Rust 下载分别需要两条不同的信任链，第二张证书可通过
`RKFLASH_DOCKER_EXTRA_CA_CERT` 传入。

## 构建 Debian 安装包

将官方 `upgrade_tool` 放入 `tools/` 后执行：

```bash
./build.sh build
```

`./build.sh package` 是同一出包流程的等价别名。

脚本强制在 Ubuntu 22.04 Docker 镜像中编译、校验并出包，即使设置了其他
`RKFLASH_UBUNTU_VERSION` 也不会改变这一基线。构建依赖和固定版本的 Rust
工具链都已位于镜像内，不会修改宿主机，也不需要 `sudo`。
正式出包使用独立的 `output/target/delivery-ubuntu-22.04/`，不会复用 24.04/26.04
开发容器生成的链接产物。
每次执行 `build`、`package`、`offline` 或 `offline-all` 都会先完整清空 `output/dist/`，避免旧版本或
旧校验文件混入本次交付。离线依赖只在临时组包目录中下载，不会把缓存留在
`output/dist/`。

生成的 `.deb` 与安全安装器分别位于
`output/dist/rkflash_<版本>_amd64.deb` 和 `output/dist/install.sh`。普通 `.deb` 遵循 Debian
依赖模型；如果目标系统缺少运行库，直接用 `apt install` 会访问已配置的软件仓库。

在当前 Ubuntu 主机安装已经生成的 `.deb`：

```bash
./build.sh install
```

也可以直接执行与 `.deb` 同目录的安装器，方便把两个文件一起交付到其他主机：

```bash
./output/dist/install.sh
```

两个入口共用同一套安装实现，且都不重新构建。安装器会把 `.deb` 以 `0644` 临时复制到 `/var/tmp` 下的 `0755` 目录，
再调用 APT，完成后清理暂存目录。这样 `_apt` 可以在沙盒中正常读取安装包，不需要放宽
用户家目录权限，也不会出现“脱离沙盒并提权为 root”的警告。不要对位于 `0750`
家目录下的 `.deb` 直接执行 `sudo apt install`，因为 APT 读取包时安装包内部的代码尚未运行。

### 完整离线安装包

需要在无网络的目标机安装时，应用 `.deb` 仍只构建一次；系统依赖必须按目标 Ubuntu
版本分别打包。一次生成三个版本：

```bash
./build.sh offline-all
```

也可以只生成一个目标版本：

```bash
RKFLASH_UBUNTU_VERSION=22.04 ./build.sh offline
RKFLASH_UBUNTU_VERSION=24.04 ./build.sh offline
RKFLASH_UBUNTU_VERSION=26.04 ./build.sh offline
```

脚本先在 Ubuntu 22.04 基线容器中构建唯一的应用 `.deb`，再由每个目标版本的 APT 解析
干净系统所需的完整运行依赖闭包，将相同 `.deb`、该版本全部依赖、SHA-256 清单和安装
说明一并打入：

```text
output/dist/rkflash_<版本>_<发行版>-<版本>_<架构>_offline.tar.gz
```

每个离线包生成后，脚本还会在对应版本的干净 Ubuntu 基础容器中以 `--network none` 禁用
网络并实际安装。断网安装、包状态、主程序、厂商工具、udev
规则和桌面入口任一验证失败，`offline`/`offline-all` 都会返回失败。

受限网络可为每个版本分别指定基础镜像和 APT 镜像；变量名中的版本点号改为下划线：

```bash
RKFLASH_UBUNTU_IMAGE_22_04=registry.example.com/ubuntu:22.04 \
RKFLASH_UBUNTU_IMAGE_24_04=registry.example.com/ubuntu:24.04 \
RKFLASH_UBUNTU_IMAGE_26_04=registry.example.com/ubuntu:26.04 \
RKFLASH_UBUNTU_APT_MIRROR=https://mirror.example.com/ubuntu \
./build.sh offline-all
```

把这个压缩包复制到离线目标机，解压后执行：

```bash
tar -xzf rkflash_*_offline.tar.gz
cd rkflash_*_offline
sha256sum --check SHA256SUMS
./install.sh
```

安装脚本会把包暂存到 `/var/tmp` 并组织成本地 `file:` 仓库，让 apt 自行安排安装顺序并
保留目标机上比包内更新的版本；软件源列表被替换为仅含该本地仓库，不会联网下载。APT 要求
目标机现有的 dpkg 状态完整；如果存在其他半安装软件包，会立即报错，但不会联网修复或下载。
卸载同样不访问仓库：

```bash
sudo apt purge rkflash
```

卸载只清除 rkflash、桌面入口和 udev 规则；共享系统运行库会保留，避免影响其他应用。

目标机已具备运行依赖、且明确不需要 APT 处理依赖时，也可以使用 `dpkg` 直接处理指定文件：

```bash
sudo dpkg --install ./output/dist/rkflash_<版本>_amd64.deb
```

`dpkg --install` 不访问软件仓库、不下载依赖，也不会为了安装 rkflash 而修复或升级其他
软件包。如果它报告缺少依赖，应改用与目标 Ubuntu 版本匹配的完整离线包。本机正常安装
优先使用 `./build.sh install`，不要直接从权限为 `0750` 的家目录调用 APT。

应用包和离线包都包含桌面菜单入口、udev 权限规则和 `upgrade_tool`，安装完成后可从应用菜单搜索
`rkflash`，或在终端运行 `rkflash`。

## 工程结构

```text
app/                         # 所有 Rust 源码统一归档在此目录
├── api/                     # 前后端唯一共享契约：能力接口、操作与状态类型
├── backend/                 # 完整后端分层；实现 api 能力端口并装配具体适配器
│   └── src/
│       ├── application/     # 门面、用例规则和按职责拆分的输入/输出端口
│       │   └── ports/       # 执行、监控、工具、日志、规划、仓储和文件系统接口
│       ├── domain/          # 后端内部执行计划；公共类型来自 api
│       └── infrastructure/
│           ├── process/     # 命令、监控、日志、终端清洗与任务运行时
│           ├── persistence/ # 工作区仓储与本地文件系统适配器
│           └── rockchip/    # upgrade_tool 命令、解析与平台配置
├── config/                  # 镜像/工作区模型与 JSON codec
├── frontend/                # egui 桌面交付层
│   └── src/desktop/
│       ├── pages/           # 下载、升级、高级功能页面
│       ├── actions.rs       # 前端动作到应用用例的转换
│       ├── chrome.rs        # 顶部导航、设备状态与运行终端外壳
│       ├── components.rs    # 通用 UI 组件
│       ├── layout.rs        # 内容驱动的响应式布局策略
│       ├── overlays.rs      # 错误、设备提示和高风险确认浮层
│       └── theme.rs         # 视觉系统与中文字体加载
└── launcher/                # rkflash 二进制；只负责装配后端与桌面前端
```

完整依赖规则和扩展方式见 [分层架构说明](docs/分层架构.md)。运行
`./build.sh architecture` 可单独检查边界，构建安装包时也会自动执行同一门禁。

## 设计原则

- `frontend` 只依赖独立的 `api` 契约，不依赖 `backend`，不构造命令行、不启动进程。
- `backend` 实现 `api` 的 `FlashService` 能力端口；具体适配器不会暴露给前端。
- `api` 和 `config` 不依赖文件系统、进程、设备访问或 UI 框架。
- 通用执行逻辑与 Rockchip 特殊配置分开；后续改为 Rust 原生 RockUSB 时，不需要重写页面。
- 不把闭源二进制提交进源码仓库。
- 所有系统构建依赖和 Rust 工具链只存在于 Docker 镜像中；构建过程不调用宿主机的
  `apt-get`、`sudo`、`cargo` 或 `rustup`。
- 没有设备、多设备、路径缺失和任务冲突都会在写入前停止。
