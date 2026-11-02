## 下载

| 平台 | 文件 | 安装 |
|---|---|---|
| Ubuntu 22.04 / 24.04 / 26.04（amd64） | `rkflash-{VERSION}-linux-amd64.deb` | 双击用“应用中心”安装，或 `sudo apt install ./rkflash-{VERSION}-linux-amd64.deb` |
| macOS 13+ Apple Silicon | `rkflash-{VERSION}-macos-arm64.dmg` | 打开后拖入“应用程序” |
| macOS 13+ Intel | `rkflash-{VERSION}-macos-x86_64.dmg` | 打开后拖入“应用程序” |

从“下载”目录用 apt 安装时可能出现一行 `N: Download is performed unsandboxed as root…` 提示，
这是因为家目录权限为 0750，属正常现象，不影响安装。

校验：`sha256sum --check --ignore-missing SHA256SUMS`（macOS 用 `shasum -a 256 -c --ignore-missing SHA256SUMS`）。

## 注意

- macOS 应用为临时签名，未经 Apple 公证，首次打开会提示“Apple 无法验证”。点“完成”（不要移到废纸篓），
  然后在“系统设置 → 隐私与安全性”底部点“仍要打开”；或在终端执行
  `xattr -dr com.apple.quarantine /Applications/RKFlash.app`。macOS 15 起右键“打开”已不能绕过此提示。
- Linux 安装包内含 Rockchip 闭源 `upgrade_tool`（来自 vicharak-in/Linux_Upgrade_Tool，已按 SHA-256 校验），
  不属于本项目开源许可范围，详见 `linux/packaging/THIRD_PARTY.md`。
- 本工具原名 rktools（Linux）/ RKFlashMac（macOS）。Linux 新包会自动替换旧的 rktools；macOS 上旧的
  `RKFlashMac.app` 不会被覆盖，确认新版可用后把它移到废纸篓即可。
