# RKFlash

面向 Intel 和 Apple Silicon macOS 的 Rockchip USB 原生烧录工具。工程提供 SwiftUI 图形界面，
并把 `rkdeveloptool 1.32-macos.4` 和 `libusb 1.0.30` 静态打包进应用，不要求用户安装
Homebrew、驱动或 Linux 虚拟机。

## 当前功能

- 每秒自动检测 Rockchip USB 设备，并在底部独立显示 Maskrom、Loader、MSC 模式。
- Maskrom 模式下载板卡对应的 `MiniLoaderAll.bin`，等待设备重新枚举。
- 切换 eMMC、SD 卡、SPI NOR。
- 从指定扇区写入 RAW、WIC、IMG、BIN 整盘镜像。
- 按 GPT/parameter 分区名写入单个分区镜像。
- 批量分区清单编辑、启用/禁用、JSON 配置保存和顺序烧录；默认按分区名写入，另有明确警告的强制地址模式。
- 原生识别任意文件名的 RKFW/RKAF 固件包，显示固件版本、Loader 版本、芯片、MD5 和分区清单。
- 整包升级自动使用包内 Loader：自动进入 Maskrom 写入 IDBlock，再重启进入 Loader 并顺序写入固件分区。
- 可选安全解包固件包，自动隔离 Loader、解包 parameter，并载入可核对的高级分区队列。
- 烧录前计算本地镜像 SHA-256。
- 读取芯片、存储器、Flash ID、Capability、当前存储和分区表。
- 下载/升级 Loader，写入 GPT/parameter，进入 Maskrom。
- 导出或擦除指定扇区，设备读写测试、全盘擦除和重启。
- 多设备安全拦截、操作中止提示、诊断日志和单行动态写入进度；进度输出经过限频，避免长时间烧录拖慢界面。
- 应用内保留完整 `rkdeveloptool` CLI，可使用 `rl`、`gpt`、`prm`、`ul`、
  `pack`、`unpack`、`tagspl` 等高级命令。

## 构建环境

- macOS 13 或更高版本。
- Intel（x86_64）或 Apple Silicon（arm64）Mac。
- Apple Command Line Tools；不需要 Homebrew，也不需要另外安装 libusb。

首次构建前安装编译工具：

```bash
xcode-select --install
```

## 本机构建与检查

首次获取工程：

```bash
git clone https://github.com/open-center/rkflash.git
cd rkflash/macos
```

已有工程时直接进入工程目录，然后构建应用：

```bash
make app
make check
```

`make app` 会自动使用当前 Mac 的处理器架构：M 系列生成 arm64，Intel Mac 生成 x86_64。
产物位于：

```text
build/RKFlash.app
```

开发阶段可直接启动构建产物：

```bash
make run
```

需要在 M 系列 Mac 上交叉检查 Intel 构建时，可以显式指定架构：

```bash
make clean
make ARCH=x86_64 app
file build/RKFlash.app/Contents/MacOS/RKFlash
codesign --verify --deep --strict build/RKFlash.app
```

交叉构建只能验证编译、文件架构和签名。`make check` 以及 USB/烧录测试应在 Intel Mac
本机执行，避免依赖 Rosetta 代为运行 x86_64 测试程序。

## 安装到“应用程序”

构建并安装到 `/Applications/RKFlash.app`：

```bash
make install
```

安装完成后不再需要通过命令启动，可从下面任一位置直接打开：

1. 启动台中点击“RKFlash”。
2. Spotlight 搜索“RKFlash”。
3. Finder → 应用程序 → RKFlash。

更新本机版本时，先退出正在运行的 RKFlash，再重新执行 `make install`。卸载时在
Finder 的“应用程序”中把 RKFlash 移到废纸篓即可；应用不会安装内核驱动或后台服务。

## 制作 DMG 安装镜像

生成可双击打开、拖入“Applications”的标准安装镜像：

```bash
make dmg
```

输出文件名包含应用版本和处理器架构，例如：

```text
build/dist/RKFlash-<版本>-arm64.dmg
build/dist/RKFlash-<版本>-x86_64.dmg
```

当前构建使用本机临时签名，适合本机开发和验证。若要把 DMG 发给其他用户并获得和正式
macOS 应用一致的 Gatekeeper 体验，需要使用 Apple Developer ID Application 证书签名，
再提交 Apple 公证；这一步需要开发者账号和证书，未包含在公开的本地构建命令中。

兼容性实现只依据 Rockchip 协议字段、容器 magic、payload 内容和设备返回值。GPT 由
`EFI PART` 签名识别，parameter 由 `PARM`、`TYPE`、`CMDLINE/mtdparts` 正向解析；文件是
`.img`、`.bin`、`.conf` 或无扩展名都不影响结果。固件文件名、目录名、板卡商品名和某一个
芯片型号不参与烧录流程判断；无法确认的格式会停止并报告，不会猜测地址或套用规则。

## Mac App Store 准备

工程包含 App Sandbox 权限、辅助工具继承权限、隐私清单、第三方许可证和商店资料草案。
本地检查沙盒构建：

```bash
make sandbox-check
make universal-check
```

账号、签名、隐私政策发布、实体设备审核材料和 GPL 分发确认见
[`AppStore/README.md`](AppStore/README.md)。`make sandbox-check` 只验证包结构与签名权限，
不能代替 Intel/Apple Silicon 真机 USB 烧录，也不能代替 App Store Connect 的上传验证。

## 完整重建

修改原生核心、Swift 界面、Info.plist 或图标后，可清理并完整重建：

```bash
make clean
make app
make check
make dmg
```

应用图标源文件为 `Resources/AppIcon-1024.png`，应用包使用
`Resources/AppIcon.icns`。构建时会把图标自动复制进 `.app` 并随整个应用签名。

## 烧录操作

1. 使用开发板的 USB OTG/下载口连接 Mac；UART 串口只用于查看日志，不能完成 RockUSB 烧录。
2. 让开发板进入 Loader 或 Maskrom，确认底部状态栏显示 `LOADER` 或 `MASKROM`。
3. Maskrom 模式选择与板卡和 DDR 配置严格匹配的 Loader，再选择镜像和目标存储器。
4. 整盘 WIC/RAW 镜像通常从扇区 `0` 写入；分区镜像选择“单分区”并填写 GPT 中的分区名。
5. 点击“开始烧录”，不要在写入期间断电或拔线。

不确定 Loader 或起始扇区时，应先核对原 Ubuntu 烧录命令，不能猜测。

## 命令行

应用内核心路径：

```bash
CORE="/Applications/RKFlash.app/Contents/MacOS/rkdeveloptool"
"$CORE" ld
"$CORE" db /path/to/MiniLoaderAll.bin
"$CORE" wl 0 /path/to/image.wic
"$CORE" rd
```

查看完整命令：

```bash
"$CORE" --help
```

## RKFW/RKAF 固件包升级

1. 选择“升级固件”，打开 RKFW 或 RKAF 格式的固件包；文件名和扩展名不限，由文件头识别格式。
2. 核对固件芯片型号、MD5 应为“校验通过”或“未附带”，并检查分区清单；开始升级时还会读取设备芯片并动态比对。
3. 选择目标存储，点击“使用包内 Loader 开始升级”。不需要另外选择 Loader 文件。
4. 应用会自动提取包内 Loader；若设备当前处于 Loader，会先切换到 Maskrom，然后写入匹配的 Loader/IDBlock 并重启回 Loader。
5. 应用随后优先写入 GPT/parameter，再按固件包清单中的目标地址写入其余分区，最后按选项测试并重启。

这类固件包传统上常命名为 `update.img`，但名称不是格式的一部分。它是 RKFW/RKAF 容器，
不是整盘 RAW 镜像；本工具会根据 `RKFW`/`RKAF` 文件头识别，并明确禁止把容器直接写到扇区 0。
MD5 失败或芯片型号不匹配时会拒绝升级。

## 验证边界

当前已在 arm64 macOS 26.6.2 上完成：

- SwiftUI 应用和静态 libusb/RockUSB 核心构建。
- Mach-O 架构、依赖、Info.plist 和临时签名检查。
- CLI 帮助、版本、无设备枚举以及设备输出解析测试。
- 合成 Rockchip RKFW/RKAF 固件的头部、MD5、Loader 版本、分区解析与实际解包测试。
- 恶意越界分区条目的拒绝测试，以及 parameter 解包内容校验。
- 原生界面启动和无设备错误流程检查。

已经通过真实 Rockchip 设备完成 Loader 枚举和芯片信息只读检查；Loader 下载、USB 模式切换、
eMMC 写入和板端启动仍属于待验证项。首次真机烧录建议先执行只读的“检测设备”和“读取芯片/存储信息”，再用
可恢复的测试板进行烧录。

## 许可证

应用与 rkdeveloptool 派生部分按 GPL-2.0-or-later 发布；libusb 按 LGPL-2.1-or-later。
具体上游版本见 [THIRD_PARTY.md](THIRD_PARTY.md)。
