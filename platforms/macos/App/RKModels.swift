import Foundation

enum FlashMode: String, CaseIterable, Identifiable {
    case wholeDisk = "整盘镜像"
    case partition = "单分区"
    case batch = "批量分区"
    case firmwareUpgrade = "升级固件"

    var id: String { rawValue }
}

struct BatchFlashItem: Identifiable, Codable, Equatable {
    var id = UUID()
    var isEnabled = true
    var partitionName = ""
    var address = ""
    var imagePath = ""
}

struct BatchFlashConfiguration: Codable {
    static let currentVersion = 1

    var version = currentVersion
    var loaderPath: String
    var storageTarget: StorageTarget
    var forceAddressWrite: Bool
    var items: [BatchFlashItem]
}

enum StorageTarget: String, CaseIterable, Identifiable, Codable {
    case emmc = "eMMC"
    case sd = "SD 卡"
    case spinor = "SPI NOR"

    var id: String { rawValue }

    var rkCode: String {
        switch self {
        case .emmc: return "1"
        case .sd: return "2"
        case .spinor: return "9"
        }
    }
}

enum RKDeviceMode: String {
    case maskrom = "Maskrom"
    case loader = "Loader"
    case msc = "MSC"
    case unknown = "Unknown"
}

struct RKDevice: Identifiable, Equatable {
    let number: Int
    let vendorID: String
    let productID: String
    let locationID: String
    let mode: RKDeviceMode

    var id: String { "\(vendorID):\(productID):\(locationID)" }

    var summary: String {
        "\(mode.rawValue) · VID \(vendorID) · PID \(productID) · 位置 \(locationID)"
    }

    static func parseList(_ output: String) -> [RKDevice] {
        let pattern = #"DevNo=(\d+)\s+Vid=0x([0-9a-fA-F]+),Pid=0x([0-9a-fA-F]+),LocationID=([0-9a-fA-F]+)\s+(\w+)"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return [] }
        let range = NSRange(output.startIndex..<output.endIndex, in: output)

        return regex.matches(in: output, range: range).compactMap { match in
            guard match.numberOfRanges == 6,
                  let numberRange = Range(match.range(at: 1), in: output),
                  let vidRange = Range(match.range(at: 2), in: output),
                  let pidRange = Range(match.range(at: 3), in: output),
                  let locationRange = Range(match.range(at: 4), in: output),
                  let modeRange = Range(match.range(at: 5), in: output),
                  let number = Int(output[numberRange]) else {
                return nil
            }

            let rawMode = String(output[modeRange]).lowercased()
            let mode: RKDeviceMode
            switch rawMode {
            case "maskrom": mode = .maskrom
            case "loader": mode = .loader
            case "msc": mode = .msc
            default: mode = .unknown
            }

            return RKDevice(
                number: number,
                vendorID: "0x" + output[vidRange].lowercased(),
                productID: "0x" + output[pidRange].lowercased(),
                locationID: String(output[locationRange]),
                mode: mode
            )
        }
    }
}

enum RKToolOutput {
    static func changedStorage(_ output: String) -> Bool {
        output.range(of: "Change Storage OK", options: .caseInsensitive) != nil
    }

    static func sanitize(_ line: String) -> String {
        line
            .replacingOccurrences(
                of: "\u{001B}\\[[0-?]*[ -/]*[@-~]",
                with: "",
                options: .regularExpression
            )
            .replacingOccurrences(of: "\u{0008}", with: "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    static func progress(from line: String) -> RKToolProgress? {
        let clean = sanitize(line)
        guard !clean.isEmpty else { return nil }

        let range = NSRange(clean.startIndex..<clean.endIndex, in: clean)
        if let match = percentRegex.firstMatch(in: clean, range: range),
           let valueRange = Range(match.range(at: 1), in: clean),
           let percent = Double(clean[valueRange]) {
            return RKToolProgress(text: clean, fraction: min(max(percent / 100, 0), 1))
        }

        if let match = amountRegex.firstMatch(in: clean, range: range),
           let totalRange = Range(match.range(at: 1), in: clean),
           let currentRange = Range(match.range(at: 2), in: clean),
           let total = Double(clean[totalRange]), total > 0,
           let current = Double(clean[currentRange]) {
            return RKToolProgress(text: clean, fraction: min(max(current / total, 0), 1))
        }
        return nil
    }

    private static let percentRegex = try! NSRegularExpression(pattern: #"\(\s*(\d{1,3}(?:\.\d+)?)\s*%\s*\)"#)
    private static let amountRegex = try! NSRegularExpression(
        pattern: #"\btotal\s+(\d+)(?:K|KB)?\s*,\s*current\s+(\d+)(?:K|KB)?\b"#,
        options: .caseInsensitive
    )
}

struct RKToolProgress: Equatable {
    let text: String
    let fraction: Double
}

enum RKToolStreamEvent: Equatable {
    case log(String)
    case progress(RKToolProgress)
}

/// Converts terminal-style output into durable log lines and one replaceable
/// progress event. Delimiters and ANSI sequences may be split across pipe reads.
final class RKToolOutputAccumulator {
    private var pending = Data()
    private var lastProgress: RKToolProgress?

    func consume(_ text: String, final: Bool = false) -> [RKToolStreamEvent] {
        consume(Data(text.utf8), final: final)
    }

    func consume(_ data: Data, final: Bool = false) -> [RKToolStreamEvent] {
        pending.append(data)
        var events: [RKToolStreamEvent] = []

        while let delimiter = pending.firstIndex(where: { $0 == 0x0d || $0 == 0x0a }) {
            let rawLine = String(decoding: pending[..<delimiter], as: UTF8.self)
            let delimiterCharacter = pending[delimiter]
            var next = pending.index(after: delimiter)
            if delimiterCharacter == 0x0d, next < pending.endIndex, pending[next] == 0x0a {
                next = pending.index(after: next)
            }
            pending.removeSubrange(..<next)
            appendEvent(for: rawLine, to: &events)
        }

        if final, !pending.isEmpty {
            appendEvent(for: String(decoding: pending, as: UTF8.self), to: &events)
            pending.removeAll(keepingCapacity: true)
        }
        return events
    }

    private func appendEvent(for rawLine: String, to events: inout [RKToolStreamEvent]) {
        let clean = RKToolOutput.sanitize(rawLine)
        guard !clean.isEmpty else { return }
        if let progress = RKToolOutput.progress(from: clean) {
            guard progress != lastProgress else { return }
            lastProgress = progress
            events.append(.progress(progress))
        } else {
            events.append(.log(clean))
        }
    }
}

enum RKParameterLayout: Equatable {
    case gpt
    case legacy
    case unknown
}

enum RKParameterFormat {
    static func layout(of text: String) -> RKParameterLayout {
        var hasPartitionCommandLine = false
        for rawLine in text.split(whereSeparator: \.isNewline) {
            let line = rawLine.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !line.hasPrefix("#"), let separator = line.firstIndex(of: ":") else { continue }
            let key = line[..<separator].trimmingCharacters(in: .whitespacesAndNewlines)
            let rawValue = line[line.index(after: separator)...]
                .trimmingCharacters(in: .whitespacesAndNewlines)
            let value = rawValue.split(whereSeparator: \.isWhitespace).first.map(String.init) ?? ""
            if key.caseInsensitiveCompare("TYPE") == .orderedSame {
                if value.caseInsensitiveCompare("GPT") == .orderedSame { return .gpt }
                if value.caseInsensitiveCompare("MTD") == .orderedSame { return .legacy }
                return .unknown
            }
            if key.caseInsensitiveCompare("CMDLINE") == .orderedSame,
               rawValue.range(of: "mtdparts=", options: .caseInsensitive) != nil {
                hasPartitionCommandLine = true
            }
        }
        return hasPartitionCommandLine ? .legacy : .unknown
    }

    static func looksLikeParameter(_ text: String) -> Bool {
        let standardKeys = Set([
            "FIRMWARE_VER", "MACHINE_MODEL", "MACHINE_ID", "MANUFACTURER",
            "MAGIC", "ATAG", "MACHINE", "CHECK_MASK", "PWR_HLD", "TYPE", "CMDLINE"
        ])
        return text.split(whereSeparator: \.isNewline).contains { rawLine in
            let line = rawLine.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !line.hasPrefix("#"), let separator = line.firstIndex(of: ":") else { return false }
            return standardKeys.contains(line[..<separator].trimmingCharacters(in: .whitespaces).uppercased())
        }
    }
}

enum RKPartitionMetadataFormat: Equatable {
    case gptImage
    case parameter(RKParameterLayout)
    case other

    private static let probeLimit = 512 * 1024 + 12

    static func inspect(path: String) throws -> RKPartitionMetadataFormat {
        let url = URL(fileURLWithPath: path)
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        let data = try handle.read(upToCount: probeLimit) ?? Data()
        return inspect(data: data)
    }

    static func inspect(data: Data) -> RKPartitionMetadataFormat {
        let gptSignature = Data("EFI PART".utf8)
        if data.count >= 520, data.subdata(in: 512..<520) == gptSignature {
            return .gptImage
        }

        var parameterData = data
        var hasWrapper = false
        if data.count >= 8, data.prefix(4) == Data("PARM".utf8) {
            hasWrapper = true
            let length = Int(data[4])
                | (Int(data[5]) << 8)
                | (Int(data[6]) << 16)
                | (Int(data[7]) << 24)
            guard length >= 0, 8 + length <= data.count else { return .parameter(.unknown) }
            parameterData = data.subdata(in: 8..<(8 + length))
        }

        guard let text = String(data: parameterData, encoding: .utf8) else {
            return hasWrapper ? .parameter(.unknown) : .other
        }
        let layout = RKParameterFormat.layout(of: text)
        if layout != .unknown || hasWrapper || RKParameterFormat.looksLikeParameter(text) {
            return .parameter(layout)
        }
        return .other
    }
}

enum RKChipIdentity {
    static func modernNumber(from code: UInt32) -> String? {
        let bytes = (0..<4).map { UInt8((code >> UInt32($0 * 8)) & 0xff) }
        guard bytes.allSatisfy({ $0 >= 0x30 && $0 <= 0x39 }) else { return nil }
        return String(bytes.reversed().map { Character(UnicodeScalar($0)) })
    }

    static func code(fromChipInfoOutput output: String) -> UInt32? {
        guard let marker = output.range(of: "Chip Info:", options: .caseInsensitive) else { return nil }
        let bytes = output[marker.upperBound...]
            .split(whereSeparator: { $0 == " " || $0 == "\t" || $0 == "\r" || $0 == "\n" })
            .compactMap { UInt8($0, radix: 16) }
        guard bytes.count >= 4 else { return nil }
        return UInt32(bytes[0])
            | (UInt32(bytes[1]) << 8)
            | (UInt32(bytes[2]) << 16)
            | (UInt32(bytes[3]) << 24)
    }

    static func displayName(for code: UInt32) -> String {
        if let modernNumber = modernNumber(from: code) { return "RK\(modernNumber)" }
        switch code {
        case 0x35: return "RK3588 / RK3588S"
        case 0x38: return "RK3566 / RK3568"
        case 0x39: return "RK3528"
        case 0x33: return "RK3399 / RK3399Pro"
        case 0x30: return "PX30 / RK3326"
        default: return String(format: "未知 (0x%08X)", code)
        }
    }

    static func compatibility(firmwareCode: UInt32, deviceCode: UInt32) -> Bool? {
        if firmwareCode == deviceCode { return true }
        guard let firmwareNumber = modernNumber(from: firmwareCode),
              let deviceNumber = modernNumber(from: deviceCode) else {
            return nil
        }
        return firmwareNumber == deviceNumber
    }
}

enum RKFlashError: LocalizedError {
    case toolMissing
    case noDevice
    case multipleDevices(Int)
    case loaderRequired
    case loaderModeRequired
    case maskromModeRequired
    case invalidFile(String)
    case invalidSector
    case invalidSectorCount
    case invalidPartition
    case firmwareContainerIsNotRaw
    case firmwareNotInspected
    case firmwareMD5Failed
    case firmwareChipMismatch(firmware: String, device: String)
    case embeddedLoaderMissing
    case unsupportedParameterFormat
    case noBatchItems
    case invalidBatchItem(Int, String)
    case unsupportedConfiguration(Int)
    case reconnectTimeout
    case modeTransitionTimeout(String)
    case cancelled
    case commandFailed(command: String, status: Int32)

    var errorDescription: String? {
        switch self {
        case .toolMissing:
            return "应用内未找到 rkdeveloptool 核心。请重新执行 make app。"
        case .noDevice:
            return "没有检测到 Rockchip USB 设备。请使用 OTG 口并进入 Loader 或 Maskrom。"
        case .multipleDevices(let count):
            return "检测到 \(count) 台设备。当前版本为防止烧错板，只允许连接一台。"
        case .loaderRequired:
            return "设备处于 Maskrom，必须选择与当前开发板芯片、DDR 和存储配置匹配的 Loader。"
        case .loaderModeRequired:
            return "该操作要求设备处于 Loader 模式。"
        case .maskromModeRequired:
            return "该操作要求设备处于 Maskrom 模式。"
        case .invalidFile(let label):
            return "\(label)不存在、为空或不可读。"
        case .invalidSector:
            return "起始扇区必须是十进制数字或 0x 开头的十六进制数。"
        case .invalidSectorCount:
            return "扇区数量必须是大于 0 的十进制数字或 0x 开头的十六进制数。"
        case .invalidPartition:
            return "请输入目标分区名，例如 boot、rootfs 或 userdata。"
        case .firmwareContainerIsNotRaw:
            return "检测到 RKFW/RKAF 固件包。它不是整盘 RAW 镜像，请使用“升级固件”。"
        case .firmwareNotInspected:
            return "请先选择并解析 Rockchip 固件包。"
        case .firmwareMD5Failed:
            return "固件包 MD5 校验失败，已拒绝升级。"
        case .firmwareChipMismatch(let firmware, let device):
            return "固件芯片型号为 \(firmware)，当前设备为 \(device)，型号不一致，已拒绝升级。"
        case .embeddedLoaderMissing:
            return "固件包中没有找到可用的内置 Loader；Maskrom 模式无法继续。"
        case .unsupportedParameterFormat:
            return "parameter 未声明可识别的 GPT 或传统分区布局，已停止烧录。"
        case .noBatchItems:
            return "批量烧录至少需要启用一个镜像条目。"
        case .invalidBatchItem(let row, let reason):
            return "批量烧录第 \(row) 行无效：\(reason)"
        case .unsupportedConfiguration(let version):
            return "不支持版本为 \(version) 的批量烧录配置。"
        case .reconnectTimeout:
            return "20 秒内没有等到设备重新枚举为 Loader。"
        case .modeTransitionTimeout(let mode):
            return "20 秒内没有等到设备重新枚举为 \(mode)。"
        case .cancelled:
            return "操作已取消。烧录中断后，请重新进入 Loader/Maskrom 再完整烧录。"
        case .commandFailed(let command, let status):
            return "命令失败（退出码 \(status)）：\(command)"
        }
    }
}
