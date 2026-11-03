import CryptoKit
import Foundation

enum RKFirmwareContainer: String {
    case rkfw = "RKFW + RKAF"
    case rkaf = "RKAF"
}

enum RKFirmwareMD5Status: String {
    case valid = "校验通过"
    case invalid = "校验失败"
    case notPresent = "未附带"
}

struct RKFirmwarePartition: Identifiable, Equatable {
    let index: Int
    let name: String
    let filename: String
    let dataOffset: UInt64
    let size: UInt64
    let nandAddress: UInt64

    var id: String { "\(index):\(name):\(dataOffset)" }
    var isReserved: Bool { filename.caseInsensitiveCompare("RESERVED") == .orderedSame }
    var isPackageFile: Bool { name.caseInsensitiveCompare("package-file") == .orderedSame }
    var isLoaderPayload: Bool {
        let normalizedName = name.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let normalizedFile = URL(fileURLWithPath: filename).lastPathComponent.lowercased()
        return ["loader", "bootloader", "miniloader", "idbloader"].contains(normalizedName)
            || normalizedFile.contains("miniloader")
            || normalizedFile == "loader.bin"
    }
}

struct RKFirmwareImageInfo {
    let sourcePath: String
    let container: RKFirmwareContainer
    let firmwareVersion: String?
    let loaderVersion: String?
    let buildTime: String?
    let chipCode: UInt32?
    let model: String
    let firmwareID: String
    let manufacturer: String
    let md5Status: RKFirmwareMD5Status
    let loaderOffset: UInt64?
    let loaderSize: UInt64?
    let rkafOffset: UInt64
    let rkafSize: UInt64
    let partitions: [RKFirmwarePartition]

    var chipName: String {
        guard let chipCode else { return "未知" }
        return RKChipIdentity.displayName(for: chipCode)
    }

    var flashablePartitions: [RKFirmwarePartition] {
        partitions.filter { !$0.isReserved && !$0.isPackageFile && !$0.isLoaderPayload && $0.size > 0 }
    }

    var embeddedLoaderPartition: RKFirmwarePartition? {
        partitions.first(where: { $0.isLoaderPayload && $0.size > 0 })
    }

    var embeddedLoaderFilename: String? {
        embeddedLoaderPartition?.filename
    }
}

struct RKFirmwareExtraction {
    let directory: URL
    let loaderPath: String?
    let items: [BatchFlashItem]
}

enum RKFirmwareFormatError: LocalizedError {
    case cannotOpen
    case unsupportedMagic
    case truncated(String)
    case invalidCount(UInt32)
    case invalidExtent(String)
    case unsafeFilename(String)
    case extractionFailed(String)

    var errorDescription: String? {
        switch self {
        case .cannotOpen:
            return "无法打开固件文件。"
        case .unsupportedMagic:
            return "不是受支持的 Rockchip RKFW/RKAF 固件。"
        case .truncated(let field):
            return "固件数据不完整：\(field)。"
        case .invalidCount(let count):
            return "RKAF 分区数量 \(count) 超出当前格式上限 16。"
        case .invalidExtent(let name):
            return "固件条目 \(name) 的偏移或长度越界。"
        case .unsafeFilename(let name):
            return "固件条目包含不安全的文件名：\(name)"
        case .extractionFailed(let name):
            return "解包固件条目失败：\(name)"
        }
    }
}

enum RKFirmwareImage {
    private static let rkfwHeaderSize: UInt64 = 0x66
    private static let rkafTableOffset: UInt64 = 0x8c
    private static let rkafEntrySize: UInt64 = 0x70
    private static let maximumParts: UInt32 = 16

    static func inspect(path: String) throws -> RKFirmwareImageInfo {
        let url = URL(fileURLWithPath: path)
        guard let handle = try? FileHandle(forReadingFrom: url) else {
            throw RKFirmwareFormatError.cannotOpen
        }
        defer { try? handle.close() }

        let fileSize = try handle.seekToEnd()
        try handle.seek(toOffset: 0)
        let magic = try read(handle, offset: 0, count: 4, field: "文件头")

        let container: RKFirmwareContainer
        var firmwareVersion: String?
        var loaderVersion: String?
        var buildTime: String?
        var chipCode: UInt32?
        var loaderOffset: UInt64?
        var loaderSize: UInt64?
        let rkafOffset: UInt64
        let rkafSize: UInt64

        if magic == Data("RKFW".utf8) {
            container = .rkfw
            let header = try read(handle, offset: 0, count: Int(rkfwHeaderSize), field: "RKFW 头")
            let headerLength = UInt64(header.le16(at: 0x04))
            guard headerLength >= rkfwHeaderSize else {
                throw RKFirmwareFormatError.truncated("RKFW 头长度")
            }

            let rawVersion = header.le32(at: 0x06)
            firmwareVersion = versionString(rawVersion)
            let year = header.le16(at: 0x0e)
            buildTime = String(
                format: "%04u-%02u-%02u %02u:%02u:%02u",
                year, header[0x10], header[0x11], header[0x12], header[0x13], header[0x14]
            )
            chipCode = header.le32(at: 0x15)
            loaderOffset = UInt64(header.le32(at: 0x19))
            loaderSize = UInt64(header.le32(at: 0x1d))
            rkafOffset = UInt64(header.le32(at: 0x21))
            let lowImageSize = UInt64(header.le32(at: 0x25))
            if header[0x37] == 0x48, header[0x38] == 0x49 {
                rkafSize = (UInt64(header.le32(at: 0x39)) << 32) | lowImageSize
            } else {
                rkafSize = lowImageSize
            }

            guard let loaderOffset, let loaderSize,
                  isExtentValid(offset: loaderOffset, size: loaderSize, limit: fileSize) else {
                throw RKFirmwareFormatError.invalidExtent("Loader")
            }
            guard isExtentValid(offset: rkafOffset, size: rkafSize, limit: fileSize) else {
                throw RKFirmwareFormatError.invalidExtent("RKAF")
            }
            loaderVersion = try inspectLoaderVersion(handle: handle, offset: loaderOffset, size: loaderSize)
        } else if magic == Data("RKAF".utf8) {
            container = .rkaf
            rkafOffset = 0
            rkafSize = fileSize
        } else {
            throw RKFirmwareFormatError.unsupportedMagic
        }

        let rkafPrefix = try read(handle, offset: rkafOffset, count: Int(rkafTableOffset), field: "RKAF 头")
        guard rkafPrefix.prefix(4) == Data("RKAF".utf8) else {
            throw RKFirmwareFormatError.unsupportedMagic
        }

        let partCount = rkafPrefix.le32(at: 0x88)
        guard partCount <= maximumParts else {
            throw RKFirmwareFormatError.invalidCount(partCount)
        }
        let tableSize = UInt64(partCount) * rkafEntrySize
        guard isExtentValid(offset: rkafTableOffset, size: tableSize, limit: rkafSize) else {
            throw RKFirmwareFormatError.truncated("RKAF 分区表")
        }
        let table = try read(
            handle,
            offset: rkafOffset + rkafTableOffset,
            count: Int(tableSize),
            field: "RKAF 分区表"
        )

        var partitions: [RKFirmwarePartition] = []
        for index in 0..<Int(partCount) {
            let base = index * Int(rkafEntrySize)
            let name = table.cString(at: base, count: 32)
            let filename = table.cString(at: base + 0x20, count: 60)
            let position = UInt64(table.le32(at: base + 0x60))
            let nandAddress = UInt64(table.le32(at: base + 0x64))
            let size = UInt64(table.le32(at: base + 0x6c))

            let hasNoPayload = position == UInt64(UInt32.max) || size == UInt64(UInt32.max) || size == 0
            if !hasNoPayload && !isExtentValid(offset: position, size: size, limit: rkafSize) {
                throw RKFirmwareFormatError.invalidExtent(name.isEmpty ? "#\(index + 1)" : name)
            }

            partitions.append(
                RKFirmwarePartition(
                    index: index,
                    name: name.isEmpty ? "part-\(index + 1)" : name,
                    filename: filename,
                    dataOffset: hasNoPayload ? 0 : rkafOffset + position,
                    size: hasNoPayload ? 0 : size,
                    nandAddress: nandAddress
                )
            )
        }

        let md5Status = try verifyMD5(
            handle: handle,
            fileSize: fileSize,
            preferredTailOffset: rkafOffset + rkafSize,
            container: container
        )

        return RKFirmwareImageInfo(
            sourcePath: path,
            container: container,
            firmwareVersion: firmwareVersion,
            loaderVersion: loaderVersion,
            buildTime: buildTime,
            chipCode: chipCode,
            model: rkafPrefix.cString(at: 0x08, count: 0x22),
            firmwareID: rkafPrefix.cString(at: 0x2a, count: 0x1e),
            manufacturer: rkafPrefix.cString(at: 0x48, count: 0x38),
            md5Status: md5Status,
            loaderOffset: loaderOffset,
            loaderSize: loaderSize,
            rkafOffset: rkafOffset,
            rkafSize: rkafSize,
            partitions: partitions
        )
    }

    static func extract(info: RKFirmwareImageInfo, to directory: URL) throws -> RKFirmwareExtraction {
        let manager = FileManager.default
        try manager.createDirectory(at: directory, withIntermediateDirectories: true)
        let sourceURL = URL(fileURLWithPath: info.sourcePath)
        guard let source = try? FileHandle(forReadingFrom: sourceURL) else {
            throw RKFirmwareFormatError.cannotOpen
        }
        defer { try? source.close() }

        var loaderPath: String?
        if let offset = info.loaderOffset, let size = info.loaderSize, size > 0 {
            let loaderURL = directory.appendingPathComponent("MiniLoaderAll.bin")
            try copy(source: source, offset: offset, size: size, destination: loaderURL)
            loaderPath = loaderURL.path
        }

        var usedNames: Set<String> = []
        var extractedPayloads: [String: URL] = [:]
        var items: [BatchFlashItem] = []
        for partition in info.partitions where !partition.isReserved && partition.size > 0 {
            if partition.isPackageFile { continue }
            if partition.isLoaderPayload, loaderPath != nil { continue }

            let payloadKey = "\(partition.dataOffset):\(partition.size)"
            let destination: URL
            if let existing = extractedPayloads[payloadKey] {
                destination = existing
            } else {
                let safeName = try safeOutputName(for: partition)
                let uniqueName = makeUniqueFilename(safeName, used: &usedNames)
                destination = directory.appendingPathComponent(uniqueName)
                try copy(source: source, offset: partition.dataOffset, size: partition.size, destination: destination)
                try unwrapParameterIfNeeded(at: destination)
                extractedPayloads[payloadKey] = destination
            }

            if partition.isLoaderPayload {
                if loaderPath == nil { loaderPath = destination.path }
            } else {
                items.append(
                    BatchFlashItem(
                        partitionName: partition.name,
                        address: String(format: "0x%08llX", partition.nandAddress),
                        imagePath: destination.path
                    )
                )
            }
        }

        let manifest: [String: Any] = [
            "source": info.sourcePath,
            "container": info.container.rawValue,
            "firmwareVersion": info.firmwareVersion ?? "",
            "loaderVersion": info.loaderVersion ?? "",
            "chip": info.chipName,
            "model": info.model,
            "md5": info.md5Status.rawValue,
            "partitions": info.partitions.map {
                [
                    "name": $0.name,
                    "filename": $0.filename,
                    "offset": String(format: "0x%llX", $0.dataOffset),
                    "size": $0.size,
                    "nandAddress": String(format: "0x%08llX", $0.nandAddress)
                ] as [String: Any]
            }
        ]
        let manifestData = try JSONSerialization.data(withJSONObject: manifest, options: [.prettyPrinted, .sortedKeys])
        try manifestData.write(to: directory.appendingPathComponent("rkflash-manifest.json"), options: .atomic)

        return RKFirmwareExtraction(directory: directory, loaderPath: loaderPath, items: items)
    }

    private static func inspectLoaderVersion(handle: FileHandle, offset: UInt64, size: UInt64) throws -> String? {
        guard size >= 14 else { return nil }
        let header = try read(handle, offset: offset, count: 14, field: "Loader 头")
        let magic = header.prefix(4)
        guard magic == Data("BOOT".utf8) || magic == Data("LDR ".utf8) else { return nil }
        let raw = header.le32(at: 0x06)
        if raw <= UInt32(UInt16.max) {
            return "\((raw >> 8) & 0xff).\(raw & 0xff)"
        }
        return versionString(raw)
    }

    private static func versionString(_ raw: UInt32) -> String {
        "\((raw >> 24) & 0xff).\((raw >> 16) & 0xff).\(raw & 0xffff)"
    }

    private static func verifyMD5(
        handle: FileHandle,
        fileSize: UInt64,
        preferredTailOffset: UInt64,
        container: RKFirmwareContainer
    ) throws -> RKFirmwareMD5Status {
        guard container == .rkfw else { return .notPresent }
        var candidates: [UInt64] = []
        if isExtentValid(offset: preferredTailOffset, size: 32, limit: fileSize) {
            candidates.append(preferredTailOffset)
        }
        if fileSize >= 32 {
            let eofTail = fileSize - 32
            if !candidates.contains(eofTail) { candidates.append(eofTail) }
        }

        var foundTail = false
        for tailOffset in candidates {
            let tail = try read(handle, offset: tailOffset, count: 32, field: "MD5")
            guard let expected = String(data: tail, encoding: .ascii), expected.isHexMD5 else { continue }
            foundTail = true
            let actual = try calculateMD5(handle: handle, length: tailOffset)
            if actual.caseInsensitiveCompare(expected) == .orderedSame { return .valid }
        }
        return foundTail ? .invalid : .notPresent
    }

    private static func calculateMD5(handle: FileHandle, length: UInt64) throws -> String {
        try handle.seek(toOffset: 0)
        var remaining = length
        var hasher = Insecure.MD5()
        while remaining > 0 {
            let count = Int(min(remaining, 1024 * 1024))
            guard let chunk = try handle.read(upToCount: count), !chunk.isEmpty else {
                throw RKFirmwareFormatError.truncated("MD5 数据")
            }
            hasher.update(data: chunk)
            remaining -= UInt64(chunk.count)
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    private static func safeOutputName(for partition: RKFirmwarePartition) throws -> String {
        let raw = partition.filename.isEmpty ? "\(partition.name).img" : partition.filename
        let normalized = raw.replacingOccurrences(of: "\\", with: "/")
        let components = normalized.split(separator: "/", omittingEmptySubsequences: true)
        guard !normalized.hasPrefix("/"), !components.contains(".."), let last = components.last else {
            throw RKFirmwareFormatError.unsafeFilename(raw)
        }
        let name = String(last)
        guard name != ".", name != "..", !name.isEmpty else {
            throw RKFirmwareFormatError.unsafeFilename(raw)
        }
        return name
    }

    private static func makeUniqueFilename(_ filename: String, used: inout Set<String>) -> String {
        if used.insert(filename).inserted { return filename }
        let url = URL(fileURLWithPath: filename)
        let base = url.deletingPathExtension().lastPathComponent
        let ext = url.pathExtension
        var counter = 2
        while true {
            let candidate = ext.isEmpty ? "\(base)-\(counter)" : "\(base)-\(counter).\(ext)"
            if used.insert(candidate).inserted { return candidate }
            counter += 1
        }
    }

    private static func unwrapParameterIfNeeded(at url: URL) throws {
        let source = try FileHandle(forReadingFrom: url)
        defer { try? source.close() }
        let fileSize = try source.seekToEnd()
        try source.seek(toOffset: 0)
        guard let header = try source.read(upToCount: 8),
              header.count == 8,
              header.prefix(4) == Data("PARM".utf8) else { return }
        let length = UInt64(header.le32(at: 4))
        guard 8 + length + 4 <= fileSize, length <= UInt64(Int.max) else {
            throw RKFirmwareFormatError.invalidExtent("parameter")
        }
        try source.seek(toOffset: 8)
        guard let contents = try source.read(upToCount: Int(length)), contents.count == Int(length) else {
            throw RKFirmwareFormatError.extractionFailed("parameter")
        }
        try contents.write(to: url, options: .atomic)
    }

    private static func copy(source: FileHandle, offset: UInt64, size: UInt64, destination: URL) throws {
        FileManager.default.createFile(atPath: destination.path, contents: nil)
        guard let output = try? FileHandle(forWritingTo: destination) else {
            throw RKFirmwareFormatError.extractionFailed(destination.lastPathComponent)
        }
        defer { try? output.close() }
        try source.seek(toOffset: offset)
        var remaining = size
        while remaining > 0 {
            let count = Int(min(remaining, 4 * 1024 * 1024))
            guard let chunk = try source.read(upToCount: count), !chunk.isEmpty else {
                throw RKFirmwareFormatError.extractionFailed(destination.lastPathComponent)
            }
            try output.write(contentsOf: chunk)
            remaining -= UInt64(chunk.count)
        }
    }

    private static func read(_ handle: FileHandle, offset: UInt64, count: Int, field: String) throws -> Data {
        try handle.seek(toOffset: offset)
        guard let data = try handle.read(upToCount: count), data.count == count else {
            throw RKFirmwareFormatError.truncated(field)
        }
        return data
    }

    private static func isExtentValid(offset: UInt64, size: UInt64, limit: UInt64) -> Bool {
        offset <= limit && size <= limit - offset
    }
}

private extension Data {
    func le16(at offset: Int) -> UInt16 {
        UInt16(self[offset]) | (UInt16(self[offset + 1]) << 8)
    }

    func le32(at offset: Int) -> UInt32 {
        UInt32(self[offset])
            | (UInt32(self[offset + 1]) << 8)
            | (UInt32(self[offset + 2]) << 16)
            | (UInt32(self[offset + 3]) << 24)
    }

    func cString(at offset: Int, count: Int) -> String {
        let bytes = self[offset..<(offset + count)].prefix { $0 != 0 }
        return String(decoding: bytes, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

private extension String {
    var isHexMD5: Bool {
        count == 32 && allSatisfy { $0.isHexDigit }
    }
}
