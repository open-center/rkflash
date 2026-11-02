import SwiftUI

@main
struct RKFlashApp: App {
    @StateObject private var model = RKFlashViewModel()

    var body: some Scene {
        WindowGroup {
            ContentView(model: model)
        }
        .windowStyle(.titleBar)
        .defaultSize(width: 980, height: 760)

        Settings {
            VStack(alignment: .leading, spacing: 8) {
                Text("RKFlash")
                    .font(.headline)
                Text("Rockchip USB 原生烧录工具，内置 rkdeveloptool 1.32-macos 与 libusb。")
                    .foregroundStyle(.secondary)
                Link(
                    "开源许可证与对应源码",
                    destination: URL(string: "https://github.com/open-center/rkflash")!
                )
            }
            .padding(24)
            .frame(width: 460)
        }
    }
}
