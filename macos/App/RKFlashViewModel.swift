import AppKit
import CryptoKit
import Foundation
import UniformTypeIdentifiers

final class RKFlashViewModel: ObservableObject {
    @Published var flashMode: FlashMode = .wholeDisk
    @Published var storageTarget: StorageTarget = .emmc
    @Published var loaderPath = ""
    @Published var imagePath = ""
    @Published var startSector = "0"
    @Published var partitionName = ""
    @Published var autoReset = true
    @Published var testAfterWrite = false
    @Published var forceAddressWrite = false
    @Published var batchItems: [BatchFlashItem] = [BatchFlashItem()]
    @Published var firmwarePath = ""
    @Published private(set) var firmwareInfo: RKFirmwareImageInfo?
    @Published private(set) var lastExtractionDirectory = ""

    @Published private(set) var devices: [RKDevice] = []
    @Published private(set) var deviceStatusText = "没有发现设备"
    @Published private(set) var operationStatusText = "就绪"
    @Published private(set) var logText = ""
    @Published private(set) var logRevision = 0
    @Published private(set) var progress: Double = 0
    @Published private(set) var liveProgressText = ""
    @Published private(set) var isBusy = false
    @Published private(set) var lastError: String?

    private let worker = DispatchQueue(label: "cn.roboticscenter.RKFlashMac.worker", qos: .userInitiated)
    private let stateLock = NSLock()
    private var activeProcess: Process?
    private var cancellationRequested = false
    private var monitorTimer: DispatchSourceTimer?
    private var monitorRequestPending = false
    private var progressBase: Double = 0
    private var progressScale: Double = 1
    private var progressEpoch = 0
    private var pendingToolProgress: (epoch: Int, text: String, value: Double)?
    private var toolProgressUpdateScheduled = false

    private var toolURL: URL? {
        if let executableDirectory = Bundle.main.executableURL?.deletingLastPathComponent() {
            let embeddedTool = executableDirectory.appendingPathComponent("rkdeveloptool")
            if FileManager.default.isExecutableFile(atPath: embeddedTool.path) { return embeddedTool }
        }
        return Bundle.main.url(forResource: "rkdeveloptool", withExtension: nil)
    }

    var canFlash: Bool {
        if flashMode == .firmwareUpgrade { return false }
        if flashMode == .batch {
            return !isBusy && batchItems.contains { $0.isEnabled && !$0.imagePath.isEmpty }
        }
        return !isBusy && !imagePath.isEmpty
    }

    var canUpgradeFirmware: Bool {
        guard !isBusy, let info = firmwareInfo else { return false }
        return info.sourcePath == firmwarePath
            && info.md5Status != .invalid
            && !info.flashablePartitions.isEmpty
    }

    var deviceStatusColorName: String {
        guard devices.count == 1 else { return devices.isEmpty ? "secondary" : "orange" }
        return devices[0].mode == .loader ? "green" : "blue"
    }

    func selectLoader() {
        if let path = chooseFile(title: "选择 Rockchip Loader", extensions: ["bin", "img"]) {
            loaderPath = path
        }
    }

    func startDeviceMonitoring() {
        guard monitorTimer == nil else { return }
        pollDevices()

        let timer = DispatchSource.makeTimerSource(queue: .main)
        timer.schedule(deadline: .now() + 1, repeating: 1)
        timer.setEventHandler { [weak self] in
            self?.pollDevices()
        }
        timer.resume()
        monitorTimer = timer
    }

    func stopDeviceMonitoring() {
        monitorTimer?.cancel()
        monitorTimer = nil
    }

    func pollDevices() {
        guard !isBusy, !monitorRequestPending else { return }
        monitorRequestPending = true

        worker.async { [weak self] in
            guard let self else { return }
            defer {
                DispatchQueue.main.async { self.monitorRequestPending = false }
            }

            do {
                let output = try self.runTool(
                    ["ld"],
                    echoCommand: false,
                    echoOutput: false,
                    allowedExitCodes: [0, 1]
                )
                self.publishDevices(RKDevice.parseList(output))
            } catch {
                self.publishDevices([])
            }
        }
    }

    func selectImage() {
        let extensions = flashMode == .wholeDisk
            ? ["img", "wic", "bin", "raw"]
            : ["img", "bin", "ext4", "erofs", "ubi"]
        if let path = chooseFile(title: flashMode == .wholeDisk ? "选择整盘镜像" : "选择分区镜像", extensions: extensions) {
            imagePath = path
        }
    }

    func selectFirmware() {
        guard let path = chooseFile(title: "选择 Rockchip 固件包", extensions: []) else { return }
        firmwarePath = path
        firmwareInfo = nil
        lastExtractionDirectory = ""
        inspectFirmware()
    }

    func inspectFirmware() {
        let path = firmwarePath
        firmwareInfo = nil
        startJob(name: "解析升级固件", clearProgress: true) { [weak self] in
            guard let self else { return }
            try self.validateFile(path, label: "升级固件")
            let info = try RKFirmwareImage.inspect(path: path)
            self.appendFirmwareInfo(info)
            DispatchQueue.main.async {
                self.firmwareInfo = info
                self.operationStatusText = "固件解析完成"
                self.progress = 1
            }
        }
    }

    func unpackFirmwareAndLoadBatch() {
        guard let info = firmwareInfo else {
            lastError = RKFlashError.firmwareNotInspected.errorDescription
            return
        }
        guard info.md5Status != .invalid else {
            lastError = RKFlashError.firmwareMD5Failed.errorDescription
            appendLog("安全拦截：固件包的 MD5 校验失败。")
            return
        }

        let panel = NSOpenPanel()
        panel.title = "选择固件解包位置"
        panel.prompt = "解包到这里"
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let parent = panel.url else { return }

        let sourceBase = URL(fileURLWithPath: info.sourcePath).deletingPathExtension().lastPathComponent
        let destination = uniqueDirectory(in: parent, baseName: "\(sourceBase)-unpacked")

        startJob(name: "解包升级固件", clearProgress: true) { [weak self] in
            guard let self else { return }
            let extraction = try RKFirmwareImage.extract(info: info, to: destination)
            guard !extraction.items.isEmpty else {
                throw RKFirmwareFormatError.extractionFailed("没有可烧录分区")
            }
            DispatchQueue.main.async {
                if let loader = extraction.loaderPath { self.loaderPath = loader }
                self.batchItems = extraction.items
                self.forceAddressWrite = false
                self.flashMode = .batch
                self.lastExtractionDirectory = extraction.directory.path
                self.operationStatusText = "已解包并载入批量分区，请核对后烧录"
                self.progress = 1
            }
            self.appendLog("固件已解包：\(extraction.directory.path)")
            self.appendLog("已载入 \(extraction.items.count) 个分区；包内 Loader 已从普通分区队列中排除。")
        }
    }

    func startFirmwareUpgrade() {
        let selectedInfo = firmwareInfo
        let selectedPath = firmwarePath
        let selectedStorage = storageTarget
        let shouldReset = autoReset
        let shouldTest = testAfterWrite

        startJob(name: "升级固件", clearProgress: true) { [weak self] in
            guard let self else { return }
            guard let info = selectedInfo, info.sourcePath == selectedPath else {
                throw RKFlashError.firmwareNotInspected
            }
            try self.validateFile(selectedPath, label: "固件包")
            guard info.md5Status != .invalid else { throw RKFlashError.firmwareMD5Failed }

            let workingDirectory = try self.makeFirmwareWorkingDirectory(for: info)
            self.publishStatus("正在准备固件包")
            let extraction = try RKFirmwareImage.extract(info: info, to: workingDirectory)
            guard let embeddedLoader = extraction.loaderPath else {
                throw RKFlashError.embeddedLoaderMissing
            }
            guard !extraction.items.isEmpty else {
                throw RKFirmwareFormatError.extractionFailed("没有可烧录分区")
            }
            DispatchQueue.main.async { self.lastExtractionDirectory = workingDirectory.path }
            self.appendLog("已自动提取包内 Loader：\(embeddedLoader)")

            var device = try self.detectSingleDevice()
            let chipOutput = try self.runTool(["rci"], allowedExitCodes: [0, 1])
            if let firmwareCode = info.chipCode,
               let deviceCode = RKChipIdentity.code(fromChipInfoOutput: chipOutput) {
                let deviceName = RKChipIdentity.displayName(for: deviceCode)
                switch RKChipIdentity.compatibility(firmwareCode: firmwareCode, deviceCode: deviceCode) {
                case false:
                    throw RKFlashError.firmwareChipMismatch(firmware: info.chipName, device: deviceName)
                case true:
                    self.appendLog("芯片核对通过：固件 \(info.chipName)，设备 \(deviceName)")
                case nil:
                    self.appendLog("警告：固件 \(info.chipName)，设备 \(deviceName)，编码格式不同，无法自动严格比对。")
                }
            } else {
                self.appendLog("警告：固件或设备未提供可比较的芯片编码，请人工核对型号。")
            }
            if device.mode == .loader {
                self.publishStatus("正在切换到 Maskrom")
                _ = try self.runTool(["rd", "3"])
                device = try self.waitForMode(.maskrom, timeout: 20)
            }
            guard device.mode == .maskrom else { throw RKFlashError.maskromModeRequired }

            // A device can still be reported as Maskrom after a loader has already
            // been downloaded. Repeating `db` in that live session may block on USB,
            // so probe storage access first and only initialize true BootROM Maskrom.
            let storageProbe = try self.runTool(
                ["cs", selectedStorage.rkCode],
                echoCommand: false,
                echoOutput: false,
                allowedExitCodes: [0, 1]
            )
            if RKToolOutput.changedStorage(storageProbe) {
                self.appendLog("检测到已就绪的 Loader 会话，跳过重复下载。")
            } else {
                self.publishStatus("正在下载包内 Loader 到内存")
                _ = try self.runTool(["db", embeddedLoader])

                self.publishStatus("正在选择 \(selectedStorage.rawValue)")
                _ = try self.runTool(["cs", selectedStorage.rkCode])
            }

            self.publishStatus("正在使用包内 Loader 写入 IDBlock")
            _ = try self.runTool(["ul", embeddedLoader])
            self.publishProgress(0.1)

            // The downloaded loader remains active while USB still reports
            // Maskrom. Rockchip's upgrade flow writes the remaining partitions
            // in this session; resetting here discards it and returns some boards
            // to true Maskrom instead of enumerating as Loader.
            self.publishStatus("正在确认 \(selectedStorage.rawValue) 写入通道")
            _ = try self.runTool(["cs", selectedStorage.rkCode])

            let orderedItems = extraction.items.enumerated().sorted {
                let leftPriority = self.firmwareItemPriority($0.element)
                let rightPriority = self.firmwareItemPriority($1.element)
                return leftPriority == rightPriority ? $0.offset < $1.offset : leftPriority < rightPriority
            }.map(\.element)
            for (index, item) in orderedItems.enumerated() {
                let normalizedName = item.partitionName.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
                self.publishStatus("正在写入 \(index + 1)/\(orderedItems.count)：\(item.partitionName)")
                self.configureProgressSegment(
                    base: 0.1 + 0.9 * Double(index) / Double(orderedItems.count),
                    scale: 0.9 / Double(orderedItems.count)
                )
                if try !self.writePartitionMetadataIfNeeded(
                    named: normalizedName,
                    path: item.imagePath
                ) {
                    guard self.isValidSector(item.address), item.address.lowercased() != "0xffffffff" else {
                        throw RKFlashError.invalidBatchItem(index + 1, "固件清单地址无效")
                    }
                    _ = try self.runTool(["wl", item.address, item.imagePath])
                }
                self.publishProgress(0.1 + 0.9 * Double(index + 1) / Double(orderedItems.count))
            }

            if shouldTest {
                self.publishStatus("正在执行设备读写测试")
                _ = try self.runTool(["td"])
            }
            if shouldReset {
                self.publishStatus("正在重启开发板")
                _ = try self.runTool(["rd"])
            }

            self.publishProgress(1)
            self.publishStatus("固件升级完成")
            self.appendLog("固件升级完成；全程自动使用包内 Loader，没有使用外部 Loader 文件。")
        }
    }

    func addBatchItem() {
        batchItems.append(BatchFlashItem())
    }

    func removeBatchItem(id: UUID) {
        batchItems.removeAll { $0.id == id }
        if batchItems.isEmpty { batchItems.append(BatchFlashItem()) }
    }

    func selectBatchImage(id: UUID) {
        guard let path = chooseFile(
            title: "选择分区镜像",
            extensions: ["img", "bin", "ext4", "erofs", "ubi", "raw"]
        ), let index = batchItems.firstIndex(where: { $0.id == id }) else { return }
        batchItems[index].imagePath = path

        if batchItems[index].partitionName.isEmpty {
            let fileName = URL(fileURLWithPath: path).deletingPathExtension().lastPathComponent
            batchItems[index].partitionName = fileName
        }
    }

    func saveBatchConfiguration() {
        let panel = NSSavePanel()
        panel.title = "保存批量烧录配置"
        panel.prompt = "保存"
        panel.nameFieldStringValue = "rockchip-flash.rkflash.json"
        panel.allowedContentTypes = [UTType.json]
        guard panel.runModal() == .OK, let url = panel.url else { return }

        let configuration = BatchFlashConfiguration(
            loaderPath: loaderPath,
            storageTarget: storageTarget,
            forceAddressWrite: forceAddressWrite,
            items: batchItems
        )
        do {
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            try encoder.encode(configuration).write(to: url, options: .atomic)
            appendLog("批量烧录配置已保存：\(url.path)")
        } catch {
            lastError = error.localizedDescription
            appendLog("保存配置失败：\(error.localizedDescription)")
        }
    }

    func loadBatchConfiguration() {
        guard let path = chooseFile(title: "加载批量烧录配置", extensions: ["json"]) else { return }
        do {
            let configuration = try JSONDecoder().decode(
                BatchFlashConfiguration.self,
                from: Data(contentsOf: URL(fileURLWithPath: path))
            )
            guard configuration.version == BatchFlashConfiguration.currentVersion else {
                throw RKFlashError.unsupportedConfiguration(configuration.version)
            }
            loaderPath = configuration.loaderPath
            storageTarget = configuration.storageTarget
            forceAddressWrite = configuration.forceAddressWrite
            batchItems = configuration.items.isEmpty ? [BatchFlashItem()] : configuration.items
            flashMode = .batch
            appendLog("批量烧录配置已加载：\(path)")
        } catch {
            let message = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
            lastError = message
            appendLog("加载配置失败：\(message)")
        }
    }

    func refreshDevices() {
        startJob(name: "检测设备", clearProgress: true) { [weak self] in
            guard let self else { return }
            let output = try self.runTool(["ld"], allowedExitCodes: [0, 1])
            let found = RKDevice.parseList(output)
            self.publishDevices(found)
            if found.isEmpty { throw RKFlashError.noDevice }
            if found.count > 1 { throw RKFlashError.multipleDevices(found.count) }
        }
    }

    func startFlash() {
        let selectedMode = flashMode
        let selectedStorage = storageTarget
        let loader = loaderPath
        let image = imagePath
        let sector = startSector
        let partition = partitionName.trimmingCharacters(in: .whitespacesAndNewlines)
        let shouldReset = autoReset
        let shouldTest = testAfterWrite
        let selectedBatchItems = batchItems.filter(\.isEnabled)
        let shouldForceAddress = forceAddressWrite

        startJob(name: "烧录固件", clearProgress: true) { [weak self] in
            guard let self else { return }

            if selectedMode == .batch {
                guard !selectedBatchItems.isEmpty else { throw RKFlashError.noBatchItems }
                for (index, item) in selectedBatchItems.enumerated() {
                    try self.validateFile(item.imagePath, label: "第 \(index + 1) 行镜像文件")
                    if shouldForceAddress {
                        guard self.isValidSector(item.address) else {
                            throw RKFlashError.invalidBatchItem(index + 1, "强制按地址写时必须填写有效地址")
                        }
                    } else if item.partitionName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                        throw RKFlashError.invalidBatchItem(index + 1, "必须填写分区名")
                    }
                }
            } else {
                try self.validateFile(image, label: "镜像文件")
                if selectedMode == .wholeDisk, self.isRockchipFirmwareContainer(image) {
                    throw RKFlashError.firmwareContainerIsNotRaw
                }
            }
            if !loader.isEmpty { try self.validateFile(loader, label: "Loader 文件") }

            if selectedMode == .wholeDisk {
                guard self.isValidSector(sector) else { throw RKFlashError.invalidSector }
            } else if selectedMode == .partition && partition.isEmpty {
                throw RKFlashError.invalidPartition
            }

            var checksum = ""
            if selectedMode == .batch {
                for (index, item) in selectedBatchItems.enumerated() {
                    let value = try self.sha256(of: item.imagePath)
                    self.appendLog("[\(index + 1)/\(selectedBatchItems.count)] \(URL(fileURLWithPath: item.imagePath).lastPathComponent) SHA-256: \(value)")
                }
            } else {
                self.appendLog("本地镜像 SHA-256 计算中…")
                checksum = try self.sha256(of: image)
                self.appendLog("SHA-256: \(checksum)")
            }

            var device = try self.detectSingleDevice()
            self.publishStatus("已检测到 \(device.mode.rawValue)")

            if device.mode == .maskrom {
                guard !loader.isEmpty else { throw RKFlashError.loaderRequired }
                self.publishStatus("正在下载 Loader")
                _ = try self.runTool(["db", loader])
                device = try self.waitForLoader(timeout: 20)
                self.publishStatus("Loader 已就绪")
            }

            guard device.mode == .loader else { throw RKFlashError.loaderModeRequired }

            self.publishStatus("正在选择 \(selectedStorage.rawValue)")
            _ = try self.runTool(["cs", selectedStorage.rkCode])

            self.publishStatus(selectedMode == .wholeDisk ? "正在写入整盘镜像" : "正在写入固件")
            if selectedMode == .wholeDisk {
                _ = try self.runTool(["wl", sector, image])
            } else if selectedMode == .partition {
                _ = try self.runTool(["wlx", partition, image])
            } else {
                for (index, item) in selectedBatchItems.enumerated() {
                    let label = shouldForceAddress ? item.address : item.partitionName
                    self.publishStatus("正在写入 \(index + 1)/\(selectedBatchItems.count)：\(label)")
                    self.configureProgressSegment(
                        base: Double(index) / Double(selectedBatchItems.count),
                        scale: 1 / Double(selectedBatchItems.count)
                    )
                    if shouldForceAddress {
                        _ = try self.runTool(["wl", item.address, item.imagePath])
                    } else {
                        if try !self.writePartitionMetadataIfNeeded(
                            named: item.partitionName,
                            path: item.imagePath
                        ) {
                            _ = try self.runTool(["wlx", item.partitionName, item.imagePath])
                        }
                    }
                    self.publishProgress(Double(index + 1) / Double(selectedBatchItems.count))
                }
            }

            if shouldTest {
                self.publishStatus("正在执行设备读写测试")
                _ = try self.runTool(["td"])
            }

            if shouldReset {
                self.publishStatus("正在重启开发板")
                _ = try self.runTool(["rd"])
            }

            self.publishProgress(1)
            self.publishStatus("烧录完成")
            if selectedMode == .batch {
                self.appendLog("批量烧录流程完成，共写入 \(selectedBatchItems.count) 个镜像。")
            } else {
                self.appendLog("烧录流程完成。镜像 SHA-256：\(checksum)")
            }
        }
    }

    func listPartitions() {
        runLoaderOperation(name: "读取分区表", arguments: ["ppt"])
    }

    func readDeviceInfo() {
        runLoaderOperation(name: "读取设备信息", arguments: ["rci", "rfi", "rid"])
    }

    func readCapability() {
        runLoaderOperation(name: "读取 Capability", arguments: ["rcb"])
    }

    func readCurrentStorage() {
        runLoaderOperation(name: "读取当前存储", arguments: ["rs"])
    }

    func testDevice() {
        runLoaderOperation(name: "设备读写测试", arguments: ["td"])
    }

    func resetDevice() {
        runLoaderOperation(name: "重启设备", arguments: ["rd"])
    }

    func enterMaskrom() {
        runLoaderCommand(name: "进入 Maskrom", command: ["rd", "3"])
    }

    func downloadLoaderNow() {
        let loader = loaderPath
        startJob(name: "下载 Boot", clearProgress: true) { [weak self] in
            guard let self else { return }
            try self.validateFile(loader, label: "Loader 文件")
            let device = try self.detectSingleDevice()
            guard device.mode == .maskrom else { throw RKFlashError.maskromModeRequired }
            _ = try self.runTool(["db", loader])
            _ = try self.waitForLoader(timeout: 20)
            self.publishStatus("Boot 下载完成，设备已进入 Loader")
        }
    }

    func upgradeLoader() {
        let loader = loaderPath
        startJob(name: "升级 Loader", clearProgress: true) { [weak self] in
            guard let self else { return }
            try self.validateFile(loader, label: "Loader 文件")
            let device = try self.detectSingleDevice()
            guard device.mode == .maskrom else { throw RKFlashError.maskromModeRequired }
            _ = try self.runTool(["ul", loader])
            self.publishStatus("Loader 升级完成")
        }
    }

    func writeGPT() {
        guard let path = chooseFile(title: "选择 GPT 镜像或 parameter 文件", extensions: []) else { return }
        runLoaderCommand(name: "写入 GPT", command: ["gpt", path])
    }

    func writeParameter() {
        guard let path = chooseFile(title: "选择 parameter 文件", extensions: []) else { return }
        runLoaderCommand(name: "写入 Parameter", command: ["prm", path])
    }

    func exportSectors(begin: String, count: String) {
        let panel = NSSavePanel()
        panel.title = "导出存储镜像"
        panel.prompt = "导出"
        panel.nameFieldStringValue = "rk-storage.img"
        panel.allowedContentTypes = [UTType.data]
        guard panel.runModal() == .OK, let destination = panel.url else { return }

        startJob(name: "导出存储镜像", clearProgress: true) { [weak self] in
            guard let self else { return }
            guard self.isValidSector(begin) else { throw RKFlashError.invalidSector }
            guard self.isValidPositiveSectorCount(count) else { throw RKFlashError.invalidSectorCount }
            let device = try self.detectSingleDevice()
            guard device.mode == .loader else { throw RKFlashError.loaderModeRequired }
            _ = try self.runTool(["rl", begin, count, destination.path])
            self.publishStatus("镜像已导出")
        }
    }

    func eraseSectors(begin: String, count: String) {
        startJob(name: "擦除指定扇区", clearProgress: true) { [weak self] in
            guard let self else { return }
            guard self.isValidSector(begin) else { throw RKFlashError.invalidSector }
            guard self.isValidPositiveSectorCount(count) else { throw RKFlashError.invalidSectorCount }
            let device = try self.detectSingleDevice()
            guard device.mode == .loader else { throw RKFlashError.loaderModeRequired }
            _ = try self.runTool(["el", begin, count])
            self.publishStatus("指定扇区擦除完成")
        }
    }

    func eraseFlash() {
        runLoaderOperation(name: "擦除存储", arguments: ["ef"])
    }

    func cancel() {
        stateLock.lock()
        cancellationRequested = true
        let process = activeProcess
        stateLock.unlock()
        process?.terminate()
        appendLog("已请求中止当前操作。")
    }

    func clearLog() {
        logText = ""
        logRevision &+= 1
        lastError = nil
    }

    private func runLoaderOperation(name: String, arguments: [String]) {
        startJob(name: name, clearProgress: true) { [weak self] in
            guard let self else { return }
            let device = try self.detectSingleDevice()
            guard device.mode == .loader else { throw RKFlashError.loaderModeRequired }
            for argument in arguments {
                _ = try self.runTool([argument])
            }
            self.publishStatus("\(name)完成")
        }
    }

    private func runLoaderCommand(name: String, command: [String]) {
        startJob(name: name, clearProgress: true) { [weak self] in
            guard let self else { return }
            let device = try self.detectSingleDevice()
            guard device.mode == .loader else { throw RKFlashError.loaderModeRequired }
            _ = try self.runTool(command)
            self.publishStatus("\(name)完成")
        }
    }

    private func startJob(name: String, clearProgress: Bool, work: @escaping () throws -> Void) {
        guard !isBusy else { return }
        isBusy = true
        lastError = nil
        operationStatusText = name
        if clearProgress {
            progress = 0
            liveProgressText = ""
        }

        stateLock.lock()
        cancellationRequested = false
        progressBase = 0
        progressScale = 1
        progressEpoch += 1
        pendingToolProgress = nil
        stateLock.unlock()

        appendLog("\n=== \(name) ===")
        worker.async { [weak self] in
            guard let self else { return }
            defer {
                self.stateLock.lock()
                self.cancellationRequested = false
                self.stateLock.unlock()
            }
            do {
                try work()
                DispatchQueue.main.async {
                    self.isBusy = false
                }
            } catch {
                let message = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
                self.appendLog("错误：\(message)")
                DispatchQueue.main.async {
                    self.lastError = message
                    self.operationStatusText = "操作失败"
                    self.isBusy = false
                }
            }
        }
    }

    private func detectSingleDevice(echo: Bool = true) throws -> RKDevice {
        let output = try runTool(["ld"], echoOutput: echo, allowedExitCodes: [0, 1])
        let found = RKDevice.parseList(output)
        publishDevices(found)
        if found.isEmpty { throw RKFlashError.noDevice }
        if found.count > 1 { throw RKFlashError.multipleDevices(found.count) }
        return found[0]
    }

    private func waitForLoader(timeout: TimeInterval) throws -> RKDevice {
        try waitForMode(.loader, timeout: timeout)
    }

    private func waitForMode(_ mode: RKDeviceMode, timeout: TimeInterval) throws -> RKDevice {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            try checkCancellation()
            Thread.sleep(forTimeInterval: 0.5)
            let output = try runTool(["ld"], echoCommand: false, echoOutput: false, allowedExitCodes: [0, 1])
            let found = RKDevice.parseList(output)
            publishDevices(found)
            if found.count > 1 { throw RKFlashError.multipleDevices(found.count) }
            if let device = found.first, device.mode == mode {
                appendLog("设备已重新枚举：\(device.summary)")
                return device
            }
        }
        if mode == .loader { throw RKFlashError.reconnectTimeout }
        throw RKFlashError.modeTransitionTimeout(mode.rawValue)
    }

    @discardableResult
    private func runTool(
        _ arguments: [String],
        echoCommand: Bool = true,
        echoOutput: Bool = true,
        allowedExitCodes: Set<Int32> = [0]
    ) throws -> String {
        guard let executable = toolURL else { throw RKFlashError.toolMissing }
        return try runProcess(
            executable: executable,
            arguments: arguments,
            echoCommand: echoCommand,
            echoOutput: echoOutput,
            allowedExitCodes: allowedExitCodes
        )
    }

    private func sha256(of path: String) throws -> String {
        let handle = try FileHandle(forReadingFrom: URL(fileURLWithPath: path))
        defer { try? handle.close() }
        var hasher = SHA256()
        while true {
            try checkCancellation()
            let data = try handle.read(upToCount: 1024 * 1024) ?? Data()
            if data.isEmpty { break }
            hasher.update(data: data)
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    private func runProcess(
        executable: URL,
        arguments: [String],
        echoCommand: Bool,
        echoOutput: Bool,
        allowedExitCodes: Set<Int32>
    ) throws -> String {
        try checkCancellation()

        let process = Process()
        let pipe = Pipe()
        let outputLock = NSLock()
        let outputAccumulator = RKToolOutputAccumulator()
        var collected = Data()
        let maximumCapturedBytes = 4 * 1024 * 1024

        process.executableURL = executable
        process.arguments = arguments
        process.standardOutput = pipe
        process.standardError = pipe

        if executable.lastPathComponent == "rkdeveloptool",
           let applicationSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first {
            let workingDirectory = applicationSupport.appendingPathComponent("RKFlashMac", isDirectory: true)
            try? FileManager.default.createDirectory(at: workingDirectory, withIntermediateDirectories: true)
            process.currentDirectoryURL = workingDirectory
        }

        if echoCommand {
            let command = ([executable.lastPathComponent] + arguments).map(displayQuoted).joined(separator: " ")
            appendLog("$ \(command)")
        }

        pipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty else { return }
            outputLock.lock()
            if data.count >= maximumCapturedBytes {
                collected = Data(data.suffix(maximumCapturedBytes))
            } else {
                let overflow = collected.count + data.count - maximumCapturedBytes
                if overflow > 0 { collected.removeFirst(overflow) }
                collected.append(data)
            }
            let events = outputAccumulator.consume(data)
            outputLock.unlock()
            self?.publishToolEvents(events, echoOutput: echoOutput)
        }

        stateLock.lock()
        activeProcess = process
        stateLock.unlock()

        do {
            try process.run()
        } catch {
            pipe.fileHandleForReading.readabilityHandler = nil
            stateLock.lock()
            activeProcess = nil
            stateLock.unlock()
            throw error
        }

        process.waitUntilExit()
        pipe.fileHandleForReading.readabilityHandler = nil
        let tail = pipe.fileHandleForReading.readDataToEndOfFile()
        outputLock.lock()
        if !tail.isEmpty {
            if tail.count >= maximumCapturedBytes {
                collected = Data(tail.suffix(maximumCapturedBytes))
            } else {
                let overflow = collected.count + tail.count - maximumCapturedBytes
                if overflow > 0 { collected.removeFirst(overflow) }
                collected.append(tail)
            }
        }
        let finalEvents = outputAccumulator.consume(tail, final: true)
        outputLock.unlock()
        publishToolEvents(finalEvents, echoOutput: echoOutput)

        stateLock.lock()
        activeProcess = nil
        let wasCancelled = cancellationRequested
        stateLock.unlock()

        if wasCancelled { throw RKFlashError.cancelled }

        outputLock.lock()
        let output = String(data: collected, encoding: .utf8) ?? ""
        outputLock.unlock()

        guard allowedExitCodes.contains(process.terminationStatus) else {
            let command = ([executable.lastPathComponent] + arguments).map(displayQuoted).joined(separator: " ")
            throw RKFlashError.commandFailed(command: command, status: process.terminationStatus)
        }
        return output
    }

    private func validateFile(_ path: String, label: String) throws {
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory),
              !isDirectory.boolValue,
              FileManager.default.isReadableFile(atPath: path),
              let attributes = try? FileManager.default.attributesOfItem(atPath: path),
              let size = attributes[.size] as? NSNumber,
              size.uint64Value > 0 else {
            throw RKFlashError.invalidFile(label)
        }
    }

    private func uniqueDirectory(in parent: URL, baseName: String) -> URL {
        let manager = FileManager.default
        var destination = parent.appendingPathComponent(baseName, isDirectory: true)
        var suffix = 2
        while manager.fileExists(atPath: destination.path) {
            destination = parent.appendingPathComponent("\(baseName)-\(suffix)", isDirectory: true)
            suffix += 1
        }
        return destination
    }

    private func makeFirmwareWorkingDirectory(for info: RKFirmwareImageInfo) throws -> URL {
        guard let applicationSupport = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first else {
            throw RKFirmwareFormatError.extractionFailed("无法访问 Application Support")
        }
        let parent = applicationSupport
            .appendingPathComponent("RKFlashMac", isDirectory: true)
            .appendingPathComponent("FirmwareCache", isDirectory: true)
        try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
        let sourceBase = URL(fileURLWithPath: info.sourcePath).lastPathComponent
        return uniqueDirectory(in: parent, baseName: "\(sourceBase)-unpacked")
    }

    private func firmwareItemPriority(_ item: BatchFlashItem) -> Int {
        if let format = try? RKPartitionMetadataFormat.inspect(path: item.imagePath), format != .other {
            return 0
        }
        let declaredRole = item.partitionName.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return ["gpt", "parameter"].contains(declaredRole) ? 0 : 1
    }

    private func writePartitionMetadataIfNeeded(named name: String, path: String) throws -> Bool {
        let format = try RKPartitionMetadataFormat.inspect(path: path)
        switch format {
        case .gptImage:
            appendLog("根据 EFI PART 签名识别为 GPT 镜像，正在写入主 GPT 和备份 GPT。")
            _ = try runTool(["gpt", path])
            _ = try runTool(["ppt"])
            return true
        case .parameter(.gpt):
            appendLog("检测到 TYPE: GPT，正在生成主 GPT 和备份 GPT。")
            _ = try runTool(["gpt", path])
            _ = try runTool(["ppt"])
            return true
        case .parameter(.legacy):
            appendLog("检测到传统 parameter 布局，正在写入 parameter 区域。")
            _ = try runTool(["prm", path])
            return true
        case .parameter(.unknown):
            throw RKFlashError.unsupportedParameterFormat
        case .other:
            let declaredRole = name.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            if ["gpt", "parameter"].contains(declaredRole) {
                throw RKFlashError.unsupportedParameterFormat
            }
            return false
        }
    }

    private func appendFirmwareInfo(_ info: RKFirmwareImageInfo) {
        appendLog("容器：\(info.container.rawValue)")
        if let version = info.firmwareVersion { appendLog("固件版本：\(version)") }
        if let loaderVersion = info.loaderVersion { appendLog("Loader 版本：\(loaderVersion)") }
        appendLog("芯片：\(info.chipName)")
        appendLog("型号：\(info.model.isEmpty ? "未标注" : info.model)")
        appendLog("MD5：\(info.md5Status.rawValue)")
        appendLog("分区条目：\(info.partitions.count)，可解包：\(info.flashablePartitions.count)")
    }

    private func isValidSector(_ value: String) -> Bool {
        if value.hasPrefix("0x") || value.hasPrefix("0X") {
            return value.count > 2 && UInt64(value.dropFirst(2), radix: 16) != nil
        }
        return !value.isEmpty && UInt64(value, radix: 10) != nil
    }

    private func isRockchipFirmwareContainer(_ path: String) -> Bool {
        guard let handle = try? FileHandle(forReadingFrom: URL(fileURLWithPath: path)) else { return false }
        defer { try? handle.close() }
        guard let magic = try? handle.read(upToCount: 4) else { return false }
        return magic == Data("RKFW".utf8) || magic == Data("RKAF".utf8)
    }

    private func isValidPositiveSectorCount(_ value: String) -> Bool {
        let parsed: UInt64?
        if value.hasPrefix("0x") || value.hasPrefix("0X") {
            parsed = UInt64(value.dropFirst(2), radix: 16)
        } else {
            parsed = UInt64(value, radix: 10)
        }
        return parsed.map { $0 > 0 } ?? false
    }

    private func checkCancellation() throws {
        stateLock.lock()
        let cancelled = cancellationRequested
        stateLock.unlock()
        if cancelled { throw RKFlashError.cancelled }
    }

    private func chooseFile(title: String, extensions: [String]) -> String? {
        let panel = NSOpenPanel()
        panel.title = title
        panel.prompt = "选择"
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        if !extensions.isEmpty {
            panel.allowedContentTypes = extensions.compactMap { UTType(filenameExtension: $0) }
        }
        return panel.runModal() == .OK ? panel.url?.path : nil
    }

    private func publishDevices(_ found: [RKDevice]) {
        DispatchQueue.main.async {
            self.devices = found
            switch found.count {
            case 0:
                self.deviceStatusText = "没有发现设备"
            case 1:
                self.deviceStatusText = "发现一个 \(found[0].mode.rawValue.uppercased()) 设备"
            default:
                self.deviceStatusText = "发现 \(found.count) 个设备，已禁止操作"
            }
        }
    }

    private func publishStatus(_ value: String) {
        DispatchQueue.main.async { self.operationStatusText = value }
    }

    private func publishProgress(_ value: Double) {
        DispatchQueue.main.async { self.progress = min(max(value, 0), 1) }
    }

    private func configureProgressSegment(base: Double, scale: Double) {
        stateLock.lock()
        progressBase = base
        progressScale = scale
        stateLock.unlock()
    }

    private func publishToolEvents(_ events: [RKToolStreamEvent], echoOutput: Bool) {
        var durableLines: [String] = []
        for event in events {
            switch event {
            case .log(let line):
                if echoOutput { durableLines.append(line) }
            case .progress(let toolProgress):
                queueToolProgress(toolProgress)
            }
        }
        if !durableLines.isEmpty { appendLog(durableLines.joined(separator: "\n")) }
    }

    private func queueToolProgress(_ toolProgress: RKToolProgress) {
        stateLock.lock()
        let base = progressBase
        let scale = progressScale
        let epoch = progressEpoch
        pendingToolProgress = (
            epoch: epoch,
            text: toolProgress.text,
            value: min(max(base + toolProgress.fraction * scale, 0), 1)
        )
        let shouldSchedule = !toolProgressUpdateScheduled
        toolProgressUpdateScheduled = true
        stateLock.unlock()

        guard shouldSchedule else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(80)) { [weak self] in
            guard let self else { return }
            self.stateLock.lock()
            let pending = self.pendingToolProgress
            self.pendingToolProgress = nil
            self.toolProgressUpdateScheduled = false
            let currentEpoch = self.progressEpoch
            self.stateLock.unlock()

            guard let pending, pending.epoch == currentEpoch else { return }
            self.liveProgressText = pending.text
            self.progress = pending.value
        }
    }

    private func appendLog(_ value: String) {
        let clean = value.replacingOccurrences(
            of: "\u{001B}\\[[0-?]*[ -/]*[@-~]",
            with: "",
            options: .regularExpression
        )
        DispatchQueue.main.async {
            if !self.logText.isEmpty && !self.logText.hasSuffix("\n") { self.logText.append("\n") }
            self.logText.append(clean)
            if !clean.hasSuffix("\n") { self.logText.append("\n") }

            let maximumCharacters = 100_000
            if self.logText.count > maximumCharacters {
                self.logText.removeFirst(self.logText.count - maximumCharacters)
            }
            self.logRevision &+= 1
        }
    }

    private func displayQuoted(_ argument: String) -> String {
        if argument.rangeOfCharacter(from: .whitespacesAndNewlines) == nil { return argument }
        return "\"\(argument.replacingOccurrences(of: "\"", with: "\\\""))\""
    }
}
