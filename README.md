# rktools

Rockchip 系列单板 USB 烧录工具，提供 Ubuntu 与 macOS 两个原生版本。

| | Linux（[`linux/`](linux/)） | macOS（[`macos/`](macos/)） |
|---|---|---|
| 支持系统 | Ubuntu 22.04 / 24.04 / 26.04，amd64 | macOS 13+，Apple Silicon 与 Intel |
| 界面 | Rust + egui | Swift + SwiftUI |
| 烧录核心 | Rockchip `upgrade_tool`（闭源，外部进程） | `rkdeveloptool` 1.32 + `libusb` 1.0.30（开源，静态内置） |
| 安装包 | `.deb` + 安全安装器，可选离线包 | `.dmg` |
| 许可证 | MIT | GPL-2.0-or-later（libusb 为 LGPL-2.1） |

两个版本的源码相互独立，各自目录下的 README 是完整的使用与构建说明。

## 下载

从 [Releases](https://github.com/open-center/rktools/releases) 下载对应平台的安装包。所有产物统一命名为
`rktools-<版本>-<平台>-<架构>.<扩展名>`，两个平台共用同一个版本号：

| 文件 | 说明 |
|---|---|
| `rktools-<版本>-linux-amd64.deb` | Ubuntu 22.04 / 24.04 / 26.04 安装包 |
| `rktools-<版本>-linux-install.sh` | Linux 安装器，见下文 |
| `rktools-<版本>-macos-arm64.dmg` | Apple Silicon Mac |
| `rktools-<版本>-macos-x86_64.dmg` | Intel Mac |
| `SHA256SUMS` | 全部文件的校验值 |

- **Ubuntu**：把 `.deb` 和 `linux-install.sh` 下载到同一目录，执行 `bash rktools-<版本>-linux-install.sh`。
  安装器会把同目录唯一的 `.deb` 暂存到 `/var/tmp` 再交给 `apt` 安装，自动补齐系统依赖，避免 APT 的
  `_apt` 用户读不到 0750 权限家目录中的安装包。安装后从应用菜单搜索 `rktools`。
- **macOS**：按处理器下载对应 `.dmg`，拖入“应用程序”。应用尚未经 Apple 公证，首次打开会提示
  “Apple 无法验证”：点“完成”，再到“系统设置 → 隐私与安全性”底部点“仍要打开”；或执行
  `xattr -dr com.apple.quarantine /Applications/RKFlashMac.app`。

## 仓库结构

```text
VERSION              # 唯一版本来源，两个平台共用
Makefile             # 统一入口：版本管理、两个平台的检查与打包
scripts/version.sh   # 把 VERSION 同步到 linux/Cargo.toml、Cargo.lock 和 macos/Info.plist
linux/               # Ubuntu 版：Rust + egui，完整说明见 linux/README.md
macos/               # macOS 版：Swift + SwiftUI，完整说明见 macos/README.md
dist/                # 本地打包输出（已忽略），文件名与 Release 一致
```

## 从源码构建

```bash
git clone https://github.com/open-center/rktools.git
cd rktools
make help
```

| 命令 | 作用 | 环境 |
|---|---|---|
| `make check` | 版本一致性 + 当前系统对应平台的检查 | — |
| `make linux-check` / `make linux` | Linux 检查 / 生成 `.deb` 与安装器 | Docker；`upgrade_tool` 放入 `linux/tools/`，见 [linux/README.md](linux/README.md) |
| `make macos-check` / `make macos` | macOS 检查 / 生成 `.dmg`（`MAC_ARCH=arm64\|x86_64`） | Xcode Command Line Tools |
| `make clean` | 清理两个平台的构建产物和 `dist/` | — |

各平台目录内原有的 `./build.sh`、`make app` 等命令仍可单独使用。

## 版本与发布

版本号只在根目录 `VERSION` 中维护。修改版本：

```bash
make set-version V=0.4.1
```

它会同步 `linux/Cargo.toml`、`linux/Cargo.lock` 和 `macos/Resources/Info.plist`；CI 用
`scripts/version.sh check` 拒绝不一致的提交。

`.github/workflows/build.yml` 在每次推送和 PR 时构建并检查两个平台：

- 推送到 `main`：自动刷新预发布 [`nightly`](https://github.com/open-center/rktools/releases/tag/nightly)，
  始终对应 main 最新提交。
- PR：只构建和检查，不发布。
- 推送 `v<VERSION>` 标签：创建正式 Release，标签必须与 `VERSION` 一致：

```bash
git tag "v$(cat VERSION)"
git push origin "v$(cat VERSION)"
```

## 许可证

本仓库按目录分别授权：`linux/` 为 [MIT](linux/LICENSE)，`macos/` 为 [GPL-2.0-or-later](macos/LICENSE)。
Linux 安装包内的 `upgrade_tool` 是 Rockchip 的闭源二进制，不属于上述任何许可范围，见
[linux/packaging/THIRD_PARTY.md](linux/packaging/THIRD_PARTY.md)。
