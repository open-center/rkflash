import CryptoKit
import Foundation

@main
struct RKModelsTests {
    static func main() {
        let output = """
        DevNo=1 Vid=0x2207,Pid=0x350b,LocationID=141 Maskrom
        DevNo=2 Vid=0x2207,Pid=0x350a,LocationID=142    Loader
        """

        let devices = RKDevice.parseList(output)
        precondition(devices.count == 2)
        precondition(devices[0].mode == .maskrom)
        precondition(devices[0].vendorID == "0x2207")
        precondition(devices[1].mode == .loader)
        precondition(devices[1].locationID == "142")
        precondition(RKDevice.parseList("not found any devices!").isEmpty)
        precondition(RKToolOutput.changedStorage("Change Storage OK."))
        precondition(!RKToolOutput.changedStorage("Change Storage failed!"))
        testToolOutputAccumulation()
        precondition(RKParameterFormat.layout(of: "TYPE: GPT\nCMDLINE: test") == .gpt)
        precondition(RKParameterFormat.layout(of: "  type : gpt   # standard layout") == .gpt)
        precondition(RKParameterFormat.layout(of: "TYPE: MTD\nCMDLINE: test") == .legacy)
        precondition(RKParameterFormat.layout(of: "CMDLINE: mtdparts=rk29xxnand:test") == .legacy)
        precondition(RKParameterFormat.layout(of: "# TYPE: GPT\nCMDLINE: test") == .unknown)

        var gptPayload = Data(count: 520)
        gptPayload.replaceSubrange(512..<520, with: Data("EFI PART".utf8))
        precondition(RKPartitionMetadataFormat.inspect(data: gptPayload) == .gptImage)
        precondition(
            RKPartitionMetadataFormat.inspect(data: Data("TYPE: GPT\nCMDLINE: test".utf8))
                == .parameter(.gpt)
        )
        precondition(
            RKPartitionMetadataFormat.inspect(data: Data("CMDLINE: mtdparts=rk29xxnand:test".utf8))
                == .parameter(.legacy)
        )
        var wrappedParameter = Data("PARM".utf8)
        let wrappedText = Data("TYPE: GPT\nCMDLINE: test".utf8)
        wrappedParameter.appendLE32(UInt32(wrappedText.count))
        wrappedParameter.append(wrappedText)
        wrappedParameter.appendLE32(0)
        precondition(RKPartitionMetadataFormat.inspect(data: wrappedParameter) == .parameter(.gpt))
        precondition(RKPartitionMetadataFormat.inspect(data: Data([0xde, 0xad, 0xbe, 0xef])) == .other)

        let configuration = BatchFlashConfiguration(
            loaderPath: "/tmp/MiniLoaderAll.bin",
            storageTarget: .emmc,
            forceAddressWrite: false,
            items: [BatchFlashItem(partitionName: "boot", address: "0x8000", imagePath: "/tmp/boot.img")]
        )
        let encoded = try! JSONEncoder().encode(configuration)
        let decoded = try! JSONDecoder().decode(BatchFlashConfiguration.self, from: encoded)
        precondition(decoded.version == BatchFlashConfiguration.currentVersion)
        precondition(decoded.storageTarget == .emmc)
        precondition(decoded.items.first?.partitionName == "boot")

        testFirmwareInspectionAndExtraction()
        testFirmwareExtentValidation()

        print("RKModelsTests: PASS")
    }

    private static func testToolOutputAccumulation() {
        let accumulator = RKToolOutputAccumulator()
        precondition(accumulator.consume("\u{001B}[1A\u{001B}[2KWrite LBA from file (2").isEmpty)
        precondition(
            accumulator.consume("4%)\r") == [
                .progress(RKToolProgress(text: "Write LBA from file (24%)", fraction: 0.24))
            ]
        )
        precondition(accumulator.consume("Write LBA from file (24%)\r").isEmpty)
        precondition(
            accumulator.consume("Download Image total 1000K, current 250K\r") == [
                .progress(RKToolProgress(text: "Download Image total 1000K, current 250K", fraction: 0.25))
            ]
        )
        precondition(accumulator.consume("Write LBA succeeded.\n") == [.log("Write LBA succeeded.")])
        precondition(accumulator.consume("final line", final: true) == [.log("final line")])

        let utf8Accumulator = RKToolOutputAccumulator()
        let utf8Line = Data("固件路径：测试.img\n".utf8)
        precondition(utf8Accumulator.consume(Data(utf8Line.prefix(2))).isEmpty)
        precondition(utf8Accumulator.consume(Data(utf8Line.dropFirst(2))) == [.log("固件路径：测试.img")])
    }

    private static func testFirmwareInspectionAndExtraction() {
        let temporaryDirectory = FileManager.default.temporaryDirectory
            .appendingPathComponent("RKFlashMacTests-\(UUID().uuidString)", isDirectory: true)
        try! FileManager.default.createDirectory(at: temporaryDirectory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: temporaryDirectory) }

        let firmwareURL = temporaryDirectory.appendingPathComponent("rk3588-release-v1.2.3.firmware")
        try! makeFirmwareImage().write(to: firmwareURL)

        let info = try! RKFirmwareImage.inspect(path: firmwareURL.path)
        precondition(info.container == .rkfw)
        precondition(info.firmwareVersion == "1.2.3")
        precondition(info.loaderVersion == "1.11")
        precondition(info.chipName == "RK3588")
        precondition(info.md5Status == .valid)
        precondition(info.partitions.count == 4)
        precondition(info.flashablePartitions.map(\.name) == ["parameter", "boot_a", "boot_b"])
        precondition(info.embeddedLoaderPartition == nil)
        precondition(info.embeddedLoaderFilename == nil)

        let loaderPartition = RKFirmwarePartition(
            index: 3,
            name: "bootloader",
            filename: "Image/MiniLoaderAll.bin",
            dataOffset: 0x1000,
            size: 4096,
            nandAddress: 0x40
        )
        precondition(loaderPartition.isLoaderPayload)

        let outputURL = temporaryDirectory.appendingPathComponent("unpacked", isDirectory: true)
        let extraction = try! RKFirmwareImage.extract(info: info, to: outputURL)
        precondition(extraction.loaderPath != nil)
        precondition(extraction.items.map(\.partitionName) == ["parameter", "boot_a", "boot_b"])
        precondition(extraction.items[1].imagePath == extraction.items[2].imagePath)
        precondition(try! Data(contentsOf: outputURL.appendingPathComponent("parameter.txt")) == Data("CMDLINE:test".utf8))
        precondition(try! Data(contentsOf: outputURL.appendingPathComponent("boot.img")) == Data([0xaa, 0xbb, 0xcc]))
        precondition(FileManager.default.fileExists(atPath: outputURL.appendingPathComponent("rkflash-manifest.json").path))

        let deviceChip = RKChipIdentity.code(fromChipInfoOutput: "Chip Info:  38 38 35 33 0 0 0 0")
        precondition(deviceChip == 0x33353838)
        precondition(RKChipIdentity.compatibility(firmwareCode: info.chipCode!, deviceCode: deviceChip!) == true)
        precondition(RKChipIdentity.compatibility(firmwareCode: 0x33353638, deviceCode: deviceChip!) == false)
        precondition(RKChipIdentity.displayName(for: 0x33353638) == "RK3568")
    }

    private static func testFirmwareExtentValidation() {
        var data = Data(count: 0x8c + 0x70)
        data.replaceSubrange(0..<4, with: Data("RKAF".utf8))
        data.putLE32(1, at: 0x88)
        data.putCString("boot", at: 0x8c, count: 32)
        data.putCString("boot.img", at: 0x8c + 0x20, count: 60)
        data.putLE32(0xfffffff0, at: 0x8c + 0x60)
        data.putLE32(0x100, at: 0x8c + 0x6c)

        let url = FileManager.default.temporaryDirectory.appendingPathComponent("RKFlashMac-invalid-\(UUID().uuidString).img")
        try! data.write(to: url)
        defer { try? FileManager.default.removeItem(at: url) }

        do {
            _ = try RKFirmwareImage.inspect(path: url.path)
            preconditionFailure("invalid extent should be rejected")
        } catch RKFirmwareFormatError.invalidExtent {
            // Expected: prevents crafted firmware from reading beyond the container.
        } catch {
            preconditionFailure("unexpected error: \(error)")
        }
    }

    private static func makeFirmwareImage() -> Data {
        let rkafHeaderAndTableSize = 0x8c + 4 * 0x70
        let packageOffset = 0x300
        let parameterOffset = 0x340
        let bootOffset = 0x380
        let rawParameter = Data("CMDLINE:test".utf8)
        var wrappedParameter = Data("PARM".utf8)
        wrappedParameter.appendLE32(UInt32(rawParameter.count))
        wrappedParameter.append(rawParameter)
        wrappedParameter.appendLE32(0)
        let bootImage = Data([0xaa, 0xbb, 0xcc])

        var rkaf = Data(count: max(rkafHeaderAndTableSize, bootOffset + bootImage.count))
        rkaf.replaceSubrange(0..<4, with: Data("RKAF".utf8))
        rkaf.putCString("RK3588", at: 0x08, count: 0x22)
        rkaf.putCString("TEST", at: 0x2a, count: 0x1e)
        rkaf.putCString("RoboticsCenter", at: 0x48, count: 0x38)
        rkaf.putLE32(4, at: 0x88)

        putPart(into: &rkaf, index: 0, name: "package-file", filename: "package-file", position: packageOffset, payload: Data("package-file".utf8))
        putPart(into: &rkaf, index: 1, name: "parameter", filename: "Image/parameter.txt", position: parameterOffset, payload: wrappedParameter)
        putPart(into: &rkaf, index: 2, name: "boot_a", filename: "Image/boot.img", position: bootOffset, payload: bootImage)
        putPart(into: &rkaf, index: 3, name: "boot_b", filename: "Image/boot.img", position: bootOffset, payload: bootImage)

        let rkafOffset = 0x200
        var firmware = Data(count: rkafOffset)
        firmware.replaceSubrange(0..<4, with: Data("RKFW".utf8))
        firmware.putLE16(0x66, at: 0x04)
        firmware.putLE32(0x01020003, at: 0x06)
        firmware.putLE16(2026, at: 0x0e)
        firmware[0x10] = 8
        firmware[0x11] = 28
        // Modern vendor images encode RK3588 as the reversed ASCII bytes "8853".
        firmware.putLE32(0x33353838, at: 0x15)
        firmware.putLE32(0x66, at: 0x19)
        firmware.putLE32(32, at: 0x1d)
        firmware.putLE32(UInt32(rkafOffset), at: 0x21)
        firmware.putLE32(UInt32(rkaf.count), at: 0x25)

        var loader = Data(count: 32)
        loader.replaceSubrange(0..<4, with: Data("BOOT".utf8))
        loader.putLE16(32, at: 0x04)
        loader.putLE32(0x0000010b, at: 0x06)
        firmware.replaceSubrange(0x66..<(0x66 + loader.count), with: loader)
        firmware.append(rkaf)

        let digest = Insecure.MD5.hash(data: firmware).map { String(format: "%02x", $0) }.joined()
        firmware.append(Data(digest.utf8))
        return firmware
    }

    private static func putPart(
        into data: inout Data,
        index: Int,
        name: String,
        filename: String,
        position: Int,
        payload: Data
    ) {
        let base = 0x8c + index * 0x70
        data.putCString(name, at: base, count: 32)
        data.putCString(filename, at: base + 0x20, count: 60)
        data.putLE32(UInt32(position), at: base + 0x60)
        data.putLE32(UInt32(0x8000 + index * 0x1000), at: base + 0x64)
        data.putLE32(UInt32(payload.count), at: base + 0x68)
        data.putLE32(UInt32(payload.count), at: base + 0x6c)
        data.replaceSubrange(position..<(position + payload.count), with: payload)
    }
}

private extension Data {
    mutating func putLE16(_ value: UInt16, at offset: Int) {
        self[offset] = UInt8(value & 0xff)
        self[offset + 1] = UInt8((value >> 8) & 0xff)
    }

    mutating func putLE32(_ value: UInt32, at offset: Int) {
        self[offset] = UInt8(value & 0xff)
        self[offset + 1] = UInt8((value >> 8) & 0xff)
        self[offset + 2] = UInt8((value >> 16) & 0xff)
        self[offset + 3] = UInt8((value >> 24) & 0xff)
    }

    mutating func appendLE32(_ value: UInt32) {
        append(UInt8(value & 0xff))
        append(UInt8((value >> 8) & 0xff))
        append(UInt8((value >> 16) & 0xff))
        append(UInt8((value >> 24) & 0xff))
    }

    mutating func putCString(_ value: String, at offset: Int, count: Int) {
        let bytes = Array(value.utf8.prefix(count - 1))
        replaceSubrange(offset..<(offset + bytes.count), with: bytes)
    }
}
