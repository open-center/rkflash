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

从 [Releases](https://github.com/open-center/rktools/releases) 下载对应平台的安装包。

- **Ubuntu**：下载 `rktools_<版本>_amd64.deb` 和 `rktools-linux-install.sh` 到同一目录，执行
  `./rktools-linux-install.sh`。安装后从应用菜单搜索 `rktools`。
- **macOS**：按处理器下载 `RKFlashMac-<版本>-arm64.dmg` 或 `-x86_64.dmg`，拖入“应用程序”。
  应用尚未经 Apple 公证，首次打开会提示“Apple 无法验证”：点“完成”，再到“系统设置 → 隐私与安全性”
  底部点“仍要打开”；或执行 `xattr -dr com.apple.quarantine /Applications/RKFlashMac.app`。

## 从源码构建

```bash
git clone https://github.com/open-center/rktools.git
```

Linux（宿主机只需要 Docker；`upgrade_tool` 需自行放入 `linux/tools/`，见 [linux/README.md](linux/README.md)）：

```bash
cd rktools/linux
./build.sh
```

macOS（只需要 Xcode Command Line Tools）：

```bash
cd rktools/macos
make app && make check
```

## 发布流程

`.github/workflows/build.yml` 在每次推送和 PR 时分别构建并检查两个平台，产物包括 `.deb`、两个架构的
`.dmg` 和 `SHA256SUMS`：

- 推送到 `main`：自动刷新预发布 [`nightly`](https://github.com/open-center/rktools/releases/tag/nightly)，
  始终对应 main 最新提交。
- PR：只构建和检查，不发布。
- 推送 `v*` 标签：创建正式 Release。

```bash
git tag v2026.11.0
git push origin v2026.11.0
```

发布标签是仓库级版本，两个应用各自的版本号仍分别记录在 `linux/Cargo.toml` 与
`macos/Resources/Info.plist`，并体现在安装包文件名中。

## 许可证

本仓库按目录分别授权：`linux/` 为 [MIT](linux/LICENSE)，`macos/` 为 [GPL-2.0-or-later](macos/LICENSE)。
Linux 安装包内的 `upgrade_tool` 是 Rockchip 的闭源二进制，不属于上述任何许可范围，见
[linux/packaging/THIRD_PARTY.md](linux/packaging/THIRD_PARTY.md)。
