# Mac App Store 上架准备

此目录保存商店资料草案。它不包含 Apple 账号、证书、API 密钥、电话号码或未公开网址。

## 当前技术状态

- Bundle ID：`cn.roboticscenter.RKFlash`（注册 App ID 前仍可更改；上传第一个构建后不要再改）。
- 最低系统：macOS 13。
- 分类：Developer Tools。
- 主应用沙盒权限：USB、用户选择文件读写。
- `rkdeveloptool`：位于 `Contents/MacOS`，只声明 App Sandbox 与继承父沙盒。
- 隐私清单：不跟踪、不联网收集数据。
- GPL/LGPL 许可证与第三方来源随应用打包，设置页提供对应源码链接。

执行本地沙盒结构检查：

```bash
make sandbox-check
```

生成并检查同时包含 Intel 与 Apple Silicon 的沙盒 Universal 2 应用：

```bash
make universal-check
```

该产物使用本机临时签名，仅用于架构、沙盒和 USB 真机测试，不能直接上传 App Store Connect。
正式上传包必须由完整 Xcode 使用开发者团队的分发身份和 Mac App Store Connect 描述文件归档。

## 上架前必须完成

1. 在本机安装完整 Xcode，并在 Xcode 设置中登录已加入 Apple Developer Program 的账号。
2. 确认 `cn.roboticscenter.RKFlash` 可注册到该团队；若不可用，在首次上传前统一修改 Bundle ID、辅助工具签名 ID 和文档。
3. 创建 App Store Connect 应用记录，填写 SKU、主要语言、分类、年龄分级、内容权利和 DSA/地区合规信息。
4. 使用 Xcode 自动签名，获得 Apple Distribution、Mac Installer Distribution 和 Mac App Store Connect 描述文件。
5. 在 Intel 与 Apple Silicon 真机上运行沙盒构建，分别验证 Maskrom、Loader、固件读取、解包、写入、校验和重启。
6. 将 `privacy-policy.md` 发布到公开 HTTPS 地址，并填写隐私政策 URL 与支持 URL。
7. 准备 macOS 截图和一段包含实体 Rockchip 开发板完整烧录流程的审核附件视频。
8. 对 GPL-2.0+ 的 App Store 分发兼容性做法律确认；对应源码必须持续公开并与提交构建精确匹配。
9. 归档、验证并先上传 TestFlight 内部测试。处理完 App Store Connect 的自动检查结果后再提交审核。

## 暂不能自动完成的账号步骤

本机没有完整 Xcode，钥匙串也没有有效的 Apple 分发签名身份。安装 Xcode、选择开发者团队、
接受协议以及最终“提交审核”必须由账号持有人完成或明确授权后操作。
