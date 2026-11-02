#include "file_format.h"

#include <algorithm>
#include <cassert>
#include <filesystem>
#include <fstream>
#include <vector>

int main()
{
    const auto root = std::filesystem::temp_directory_path() / "RKFlashMac-file-format-tests";
    std::filesystem::remove_all(root);
    const auto misleadingDirectory = root / "release.img-unpacked";
    std::filesystem::create_directories(misleadingDirectory);

    const auto parameter = misleadingDirectory / "parameter";
    std::ofstream(parameter) << "TYPE: GPT\nCMDLINE: test\n";
    assert(!rk::fileHasGPTHeader(parameter.c_str()));

    const auto binaryGPT = root / "partition-table-without-extension";
    std::vector<unsigned char> bytes(520, 0);
    const unsigned char signature[] = {'E', 'F', 'I', ' ', 'P', 'A', 'R', 'T'};
    std::copy(std::begin(signature), std::end(signature), bytes.begin() + 512);
    std::ofstream(binaryGPT, std::ios::binary).write(
        reinterpret_cast<const char *>(bytes.data()),
        static_cast<std::streamsize>(bytes.size())
    );
    assert(rk::fileHasGPTHeader(binaryGPT.c_str()));

    const auto truncated = root / "truncated.img";
    std::ofstream(truncated, std::ios::binary) << "EFI PART";
    assert(!rk::fileHasGPTHeader(truncated.c_str()));

    std::filesystem::remove_all(root);
    return 0;
}
