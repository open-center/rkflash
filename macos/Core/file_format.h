#pragma once

#include <cstdint>
#include <cstdio>
#include <cstring>

namespace rk {

inline bool fileHasGPTHeader(const char *path)
{
    if (!path)
        return false;

    FILE *file = fopen(path, "rb");
    if (!file)
        return false;

    constexpr long kGPTHeaderOffset = 512;
    constexpr std::uint8_t kGPTSignature[] = {'E', 'F', 'I', ' ', 'P', 'A', 'R', 'T'};
    std::uint8_t signature[sizeof(kGPTSignature)] = {};
    const bool valid = fseek(file, kGPTHeaderOffset, SEEK_SET) == 0
        && fread(signature, 1, sizeof(signature), file) == sizeof(signature)
        && memcmp(signature, kGPTSignature, sizeof(signature)) == 0;
    fclose(file);
    return valid;
}

} // namespace rk
