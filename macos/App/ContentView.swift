import SwiftUI

struct ContentView: View {
    @ObservedObject var model: RKFlashViewModel
    @State private var confirmErase = false
    @State private var confirmCancel = false
    @State private var confirmUpgradeLoader = false
    @State private var confirmEnterMaskrom = false
    @State private var confirmEraseSectors = false
    @State private var confirmFirmwareUpgrade = false
    @State private var advancedBeginSector = "0"
    @State private var advancedSectorCount = "1"

    var body: some View {
        VStack(spacing: 14) {
            deviceHeader

            HStack(alignment: .top, spacing: 14) {
                flashPanel
                    .frame(minWidth: 540)
                maintenancePanel
                    .frame(width: 250)
            }

            progressPanel
            logPanel
            deviceStatusBar
        }
        .padding(18)
        .frame(minWidth: 900, minHeight: 700)
        .onAppear { model.startDeviceMonitoring() }
        .onDisappear { model.stopDeviceMonitoring() }
        .alert("确认擦除整个存储器？", isPresented: $confirmErase) {
            Button("取消", role: .cancel) {}
            Button("擦除", role: .destructive) { model.eraseFlash() }
        } message: {
            Text("该操作会删除当前所选存储器上的全部数据，无法撤销。")
        }
        .alert("中止正在进行的操作？", isPresented: $confirmCancel) {
            Button("继续烧录", role: .cancel) {}
            Button("立即中止", role: .destructive) { model.cancel() }
        } message: {
            Text("烧录过程中止可能使设备无法启动，需要重新进入 Loader 或 Maskrom 后完整烧录。")
        }
        .alert("确认升级 Loader？", isPresented: $confirmUpgradeLoader) {
            Button("取消", role: .cancel) {}
            Button("升级", role: .destructive) { model.upgradeLoader() }
        } message: {
            Text("Loader 必须与当前开发板芯片、DDR 和存储配置匹配。错误 Loader 可能导致设备无法正常进入 Loader 模式。")
        }
        .alert("确认进入 Maskrom？", isPresented: $confirmEnterMaskrom) {
            Button("取消", role: .cancel) {}
            Button("进入 Maskrom") { model.enterMaskrom() }
        } message: {
            Text("设备会断开并重新枚举，底部状态栏随后应显示 MASKROM。")
        }
        .alert("确认擦除指定扇区？", isPresented: $confirmEraseSectors) {
            Button("取消", role: .cancel) {}
            Button("擦除", role: .destructive) {
                model.eraseSectors(begin: advancedBeginSector, count: advancedSectorCount)
            }
        } message: {
            Text("将从扇区 \(advancedBeginSector) 开始擦除 \(advancedSectorCount) 个扇区，无法撤销。")
        }
        .alert("确认升级整个固件？", isPresented: $confirmFirmwareUpgrade) {
            Button("取消", role: .cancel) {}
            Button("开始升级", role: .destructive) { model.startFirmwareUpgrade() }
        } message: {
            Text("工具会先核对固件与设备芯片，再自动进入 Maskrom，使用固件包内的 Loader 写入 IDBlock，并在当前加载会话中继续写入固件分区。")
        }
    }

    private var deviceHeader: some View {
        HStack(spacing: 12) {
            Image(systemName: model.devices.count == 1 ? "cpu.fill" : "cpu")
                .font(.system(size: 26))
                .foregroundStyle(statusColor)
            VStack(alignment: .leading, spacing: 3) {
                Text("RK 原生烧录工具")
                    .font(.title2.bold())
                Text(model.operationStatusText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            Spacer()
            Button {
                model.refreshDevices()
            } label: {
                Label("检测设备", systemImage: "arrow.clockwise")
            }
            .disabled(model.isBusy)
        }
    }

    private var statusColor: Color {
        switch model.deviceStatusColorName {
        case "green": return .green
        case "blue": return .blue
        case "orange": return .orange
        default: return .secondary
        }
    }

    private var flashPanel: some View {
        GroupBox("固件烧录") {
            VStack(alignment: .leading, spacing: 12) {
                Picker("烧录方式", selection: $model.flashMode) {
                    ForEach(FlashMode.allCases) { mode in
                        Text(mode.rawValue).tag(mode)
                    }
                }
                .pickerStyle(.segmented)

                if model.flashMode != .firmwareUpgrade {
                    pathRow(title: "Rockchip Loader", path: model.loaderPath, action: model.selectLoader)
                    Text("设备已在 Loader 模式时可不选；Maskrom 模式必须选择与板卡匹配的 Loader。")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                if model.flashMode == .batch {
                    batchEditor
                } else if model.flashMode == .firmwareUpgrade {
                    firmwareUpgradeEditor
                } else {
                    pathRow(title: model.flashMode == .wholeDisk ? "整盘镜像" : "分区镜像", path: model.imagePath, action: model.selectImage)
                }

                if model.flashMode != .firmwareUpgrade {
                    HStack(spacing: 16) {
                        Picker("目标存储", selection: $model.storageTarget) {
                            ForEach(StorageTarget.allCases) { target in
                                Text(target.rawValue).tag(target)
                            }
                        }
                        .frame(width: 210)

                        if model.flashMode == .wholeDisk {
                            TextField("起始扇区", text: $model.startSector)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 180)
                        } else if model.flashMode == .partition {
                            TextField("分区名，例如 boot", text: $model.partitionName)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 220)
                        }
                    }

                    HStack(spacing: 20) {
                        Toggle("完成后重启", isOn: $model.autoReset)
                        Toggle("写入后设备读写测试", isOn: $model.testAfterWrite)
                    }
                    .toggleStyle(.checkbox)

                    HStack {
                        Button {
                            model.startFlash()
                        } label: {
                            Label(model.flashMode == .batch ? "开始批量烧录" : "开始烧录", systemImage: "externaldrive.badge.arrow.down")
                                .frame(minWidth: 110)
                        }
                        .buttonStyle(.borderedProminent)
                        .controlSize(.large)
                        .disabled(!model.canFlash)

                        if model.isBusy {
                            Button(role: .destructive) {
                                confirmCancel = true
                            } label: {
                                Label("中止", systemImage: "stop.fill")
                            }
                        }
                    }
                }
            }
            .padding(8)
        }
    }

    private var maintenancePanel: some View {
        GroupBox("设备维护") {
            VStack(alignment: .leading, spacing: 10) {
                maintenanceButton("读取芯片/存储信息", icon: "info.circle", action: model.readDeviceInfo)
                maintenanceButton("读取分区表", icon: "list.bullet.rectangle", action: model.listPartitions)
                maintenanceButton("设备读写测试", icon: "checkmark.shield", action: model.testDevice)
                maintenanceButton("重启开发板", icon: "restart", action: model.resetDevice)

                DisclosureGroup("高级功能") {
                    VStack(alignment: .leading, spacing: 8) {
                        maintenanceButton("读取 Capability", icon: "waveform.path.ecg", action: model.readCapability)
                        maintenanceButton("读取当前存储", icon: "externaldrive", action: model.readCurrentStorage)

                        Button {
                            model.downloadLoaderNow()
                        } label: {
                            Label("下载 Boot", systemImage: "arrow.down.to.line")
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .disabled(model.isBusy || model.loaderPath.isEmpty)

                        Button {
                            confirmUpgradeLoader = true
                        } label: {
                            Label("升级 Loader", systemImage: "arrow.up.circle")
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .disabled(model.isBusy || model.loaderPath.isEmpty)

                        maintenanceButton("写入 GPT", icon: "square.grid.3x3", action: model.writeGPT)
                        maintenanceButton("写入 Parameter", icon: "doc.badge.gearshape", action: model.writeParameter)

                        Button {
                            confirmEnterMaskrom = true
                        } label: {
                            Label("进入 Maskrom", systemImage: "cpu")
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .disabled(model.isBusy)

                        Divider()

                        TextField("起始扇区", text: $advancedBeginSector)
                            .textFieldStyle(.roundedBorder)
                        TextField("扇区数量", text: $advancedSectorCount)
                            .textFieldStyle(.roundedBorder)

                        HStack {
                            Button("导出") {
                                model.exportSectors(begin: advancedBeginSector, count: advancedSectorCount)
                            }
                            Button("擦除", role: .destructive) {
                                confirmEraseSectors = true
                            }
                        }
                        .disabled(model.isBusy)
                    }
                    .padding(.top, 6)
                }

                Divider()

                Button(role: .destructive) {
                    confirmErase = true
                } label: {
                    Label("擦除整个存储器", systemImage: "trash")
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .disabled(model.isBusy)

                Divider()

                Text("安全限制")
                    .font(.caption.bold())
                Text("同时检测到多台 Rockchip 设备时会拒绝执行，避免烧错目标。")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .padding(8)
        }
    }

    private var batchEditor: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("启用")
                    .frame(width: 34)
                Text("分区名")
                    .frame(width: 100, alignment: .leading)
                Text("地址")
                    .frame(width: 100, alignment: .leading)
                Text("镜像路径")
                Spacer()
            }
            .font(.caption)
            .foregroundStyle(.secondary)

            ScrollView {
                LazyVStack(spacing: 6) {
                    ForEach($model.batchItems) { $item in
                        HStack(spacing: 7) {
                            Toggle("", isOn: $item.isEnabled)
                                .labelsHidden()
                                .toggleStyle(.checkbox)
                                .frame(width: 34)

                            TextField("boot", text: $item.partitionName)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 100)

                            TextField("0x00004000", text: $item.address)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 100)

                            Text(item.imagePath.isEmpty ? "尚未选择" : item.imagePath)
                                .lineLimit(1)
                                .truncationMode(.middle)
                                .foregroundStyle(item.imagePath.isEmpty ? .secondary : .primary)
                                .frame(maxWidth: .infinity, alignment: .leading)

                            Button("选择…") {
                                model.selectBatchImage(id: item.id)
                            }
                            .disabled(model.isBusy)

                            Button(role: .destructive) {
                                model.removeBatchItem(id: item.id)
                            } label: {
                                Image(systemName: "minus.circle")
                            }
                            .buttonStyle(.plain)
                            .disabled(model.isBusy)
                        }
                    }
                }
            }
            .frame(minHeight: 90, maxHeight: 185)

            HStack(spacing: 12) {
                Button {
                    model.addBatchItem()
                } label: {
                    Label("添加分区", systemImage: "plus")
                }
                .disabled(model.isBusy)

                Button("加载配置…", action: model.loadBatchConfiguration)
                    .disabled(model.isBusy)
                Button("保存配置…", action: model.saveBatchConfiguration)
                    .disabled(model.isBusy)

                Spacer()

                Toggle("强制按地址写", isOn: $model.forceAddressWrite)
                    .toggleStyle(.checkbox)
                    .disabled(model.isBusy)
            }

            Text(model.forceAddressWrite
                 ? "将忽略分区名，按每行地址写入；地址必须与当前产品分区表一致。"
                 : "默认按设备分区名写入，地址仅作记录，不参与烧录。")
                .font(.caption)
                .foregroundStyle(model.forceAddressWrite ? .orange : .secondary)
        }
        .padding(.vertical, 2)
    }

    private var firmwareUpgradeEditor: some View {
        VStack(alignment: .leading, spacing: 10) {
            pathRow(title: "固件包", path: model.firmwarePath, action: model.selectFirmware)

            if let info = model.firmwareInfo {
                Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 5) {
                    GridRow {
                        firmwareLabel("容器")
                        Text(info.container.rawValue)
                        firmwareLabel("固件版本")
                        Text(info.firmwareVersion ?? "未标注")
                    }
                    GridRow {
                        firmwareLabel("芯片")
                        Text(info.chipName)
                        firmwareLabel("Loader 版本")
                        Text(info.loaderVersion ?? "未识别")
                    }
                    GridRow {
                        firmwareLabel("型号")
                        Text(info.model.isEmpty ? "未标注" : info.model)
                        firmwareLabel("MD5")
                        Text(info.md5Status.rawValue)
                            .foregroundStyle(info.md5Status == .invalid ? .red : .primary)
                    }
                    GridRow {
                        firmwareLabel("固件 ID")
                        Text(info.firmwareID.isEmpty ? "未标注" : info.firmwareID)
                        firmwareLabel("厂商")
                        Text(info.manufacturer.isEmpty ? "未标注" : info.manufacturer)
                    }
                    GridRow {
                        firmwareLabel("构建时间")
                        Text(info.buildTime ?? "未标注")
                        firmwareLabel("包内 Loader")
                        Text(info.embeddedLoaderFilename ?? "RKFW 内置 Loader")
                    }
                }
                .font(.caption)

                HStack {
                    Text("可烧录分区")
                        .frame(width: 100, alignment: .leading)
                    Text("文件")
                        .frame(maxWidth: .infinity, alignment: .leading)
                    Text("地址")
                        .frame(width: 95, alignment: .leading)
                    Text("大小")
                        .frame(width: 80, alignment: .trailing)
                }
                .font(.caption.bold())
                .foregroundStyle(.secondary)

                ScrollView {
                    LazyVStack(spacing: 4) {
                        if let loader = info.embeddedLoaderPartition {
                            firmwarePartitionRow(
                                name: "Loader",
                                filename: loader.filename,
                                address: "IDBlock",
                                size: loader.size
                            )
                        }
                        ForEach(info.flashablePartitions) { partition in
                            firmwarePartitionRow(
                                name: partition.name,
                                filename: partition.filename,
                                address: String(format: "0x%08llX", partition.nandAddress),
                                size: partition.size
                            )
                        }
                    }
                }
                .frame(minHeight: 80, maxHeight: 155)

                HStack {
                    Button("重新解析", action: model.inspectFirmware)
                        .disabled(model.isBusy || model.firmwarePath.isEmpty)
                    Button {
                        model.unpackFirmwareAndLoadBatch()
                    } label: {
                        Label("解包到高级分区模式", systemImage: "shippingbox.and.arrow.backward")
                    }
                    .disabled(model.isBusy || info.md5Status == .invalid)
                }

                Divider()

                HStack(spacing: 16) {
                    Picker("目标存储", selection: $model.storageTarget) {
                        ForEach(StorageTarget.allCases) { target in
                            Text(target.rawValue).tag(target)
                        }
                    }
                    .frame(width: 210)

                    Toggle("完成后重启", isOn: $model.autoReset)
                    Toggle("写入后测试", isOn: $model.testAfterWrite)
                }
                .toggleStyle(.checkbox)

                Button {
                    confirmFirmwareUpgrade = true
                } label: {
                    Label("使用包内 Loader 开始升级", systemImage: "externaldrive.badge.arrow.down")
                        .frame(minWidth: 190)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .disabled(!model.canUpgradeFirmware)
            } else {
                Text("选择任意文件名的 RKFW/RKAF 固件包后，将显示芯片、版本、内置 Loader、MD5 和分区清单。")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Text("无需另外选择 Loader：工具会自动进入 Maskrom，并使用固件包内的 Loader 写入 IDBlock；固件包也不会被当作 RAW 镜像写到扇区 0。")
                .font(.caption)
                .foregroundStyle(.orange)
        }
        .padding(.vertical, 2)
    }

    private func firmwarePartitionRow(name: String, filename: String, address: String, size: UInt64) -> some View {
        HStack {
            Text(name)
                .frame(width: 100, alignment: .leading)
            Text(filename)
                .lineLimit(1)
                .truncationMode(.middle)
                .frame(maxWidth: .infinity, alignment: .leading)
            Text(address)
                .font(.system(.caption2, design: .monospaced))
                .frame(width: 95, alignment: .leading)
            Text(ByteCountFormatter.string(fromByteCount: Int64(size), countStyle: .file))
                .frame(width: 80, alignment: .trailing)
        }
        .font(.caption)
    }

    private func firmwareLabel(_ value: String) -> some View {
        Text(value)
            .foregroundStyle(.secondary)
            .frame(width: 72, alignment: .leading)
    }

    private var progressPanel: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(spacing: 12) {
                ProgressView(value: model.progress)
                    .progressViewStyle(.linear)
                Text("\(Int(model.progress * 100))%")
                    .font(.system(.caption, design: .monospaced))
                    .frame(width: 38, alignment: .trailing)
            }
            if !model.liveProgressText.isEmpty {
                Text(model.liveProgressText)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            if let error = model.lastError {
                Label(error, systemImage: "exclamationmark.triangle.fill")
                    .font(.caption)
                    .foregroundStyle(.red)
                    .lineLimit(2)
            }
        }
    }

    private var logPanel: some View {
        GroupBox {
            ScrollViewReader { proxy in
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        Text(model.logText.isEmpty ? "操作日志会显示在这里。" : model.logText)
                            .font(.system(size: 11, design: .monospaced))
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .topLeading)
                            .padding(8)
                        Color.clear
                            .frame(height: 1)
                            .id("log-bottom")
                    }
                }
                .onChange(of: model.logRevision) { _ in
                    DispatchQueue.main.async {
                        proxy.scrollTo("log-bottom", anchor: .bottom)
                    }
                }
            }
            .background(Color(nsColor: .textBackgroundColor))
            .clipShape(RoundedRectangle(cornerRadius: 5))
        } label: {
            HStack {
                Text("日志")
                Spacer()
                Button("清空") { model.clearLog() }
                    .buttonStyle(.plain)
            }
        }
        .frame(maxHeight: .infinity)
    }

    private var deviceStatusBar: some View {
        HStack(spacing: 9) {
            Circle()
                .fill(statusColor)
                .frame(width: 9, height: 9)
            Text(model.deviceStatusText)
                .font(.system(.body, design: .rounded).weight(.semibold))

            if let device = model.devices.first, model.devices.count == 1 {
                Text("VID \(device.vendorID) · PID \(device.productID) · 位置 \(device.locationID)")
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
            }

            Spacer()

            if model.isBusy {
                ProgressView()
                    .controlSize(.small)
                Text(model.operationStatusText)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                Text("自动检测中")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 9)
        .background(Color(nsColor: .controlBackgroundColor))
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color(nsColor: .separatorColor), lineWidth: 0.5)
        )
    }

    private func pathRow(title: String, path: String, action: @escaping () -> Void) -> some View {
        HStack {
            Text(title)
                .frame(width: 100, alignment: .leading)
            Text(path.isEmpty ? "尚未选择" : path)
                .lineLimit(1)
                .truncationMode(.middle)
                .foregroundStyle(path.isEmpty ? .secondary : .primary)
                .frame(maxWidth: .infinity, alignment: .leading)
            Button("选择…", action: action)
                .disabled(model.isBusy)
        }
    }

    private func maintenanceButton(_ title: String, icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Label(title, systemImage: icon)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .disabled(model.isBusy)
    }
}
