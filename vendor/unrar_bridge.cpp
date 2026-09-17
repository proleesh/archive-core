#include "../include/unrar_bridge.h"
#include "unrar/dll.hpp"

#include <cstdint>
#include <cstring>
#include <string>

static std::string wide_to_utf8(const wchar_t *value)
{
    if (value == nullptr || *value == L'\0')
    {
        return {};
    }

    std::string result;

    while (*value != L'\0')
    {
        uint32_t cp = static_cast<uint32_t>(*value++);

        if (cp <= 0x7F)
        {
            result.push_back(static_cast<char>(cp));
        }
        else if (cp <= 0x7FF)
        {
            result.push_back(
                static_cast<char>(0xC0 | (cp >> 6)));
            result.push_back(
                static_cast<char>(0x80 | (cp & 0x3F)));
        }
        else if (cp <= 0xFFFF)
        {
            result.push_back(
                static_cast<char>(0xE0 | (cp >> 12)));
            result.push_back(
                static_cast<char>(
                    0x80 | ((cp >> 6) & 0x3F)));
            result.push_back(
                static_cast<char>(0x80 | (cp & 0x3F)));
        }
        else
        {
            result.push_back(
                static_cast<char>(0xF0 | (cp >> 18)));
            result.push_back(
                static_cast<char>(
                    0x80 | ((cp >> 12) & 0x3F)));
            result.push_back(
                static_cast<char>(
                    0x80 | ((cp >> 6) & 0x3F)));
            result.push_back(
                static_cast<char>(0x80 | (cp & 0x3F)));
        }
    }

    return result;
}

extern "C" int32_t arkive_rar_list(
    const char *path,
    arkive_rar_entry_callback callback,
    void *context)
{
    if (path == nullptr || callback == nullptr)
    {
        return ERAR_BAD_DATA;
    }

    RAROpenArchiveDataEx open_data{};
    open_data.ArcName =
        const_cast<char *>(path);

    open_data.OpenMode = RAR_OM_LIST;

    HANDLE archive =
        RAROpenArchiveEx(&open_data);

    if (archive == nullptr)
    {
        return static_cast<int32_t>(
            open_data.OpenResult);
    }

    int32_t result = ERAR_SUCCESS;

    for (;;)
    {
        RARHeaderDataEx header{};

        int code =
            RARReadHeaderEx(
                archive,
                &header);

        if (code == ERAR_END_ARCHIVE)
        {
            result = ERAR_SUCCESS;
            break;
        }

        if (code != ERAR_SUCCESS)
        {
            result = code;
            break;
        }

        std::string name;

        if (header.FileNameW[0] != L'\0')
        {
            name = wide_to_utf8(
                header.FileNameW);
        }
        else
        {
            name = header.FileName;
        }

        uint64_t size =
            (static_cast<uint64_t>(
                 header.UnpSizeHigh)
             << 32) |
            header.UnpSize;

        uint64_t compressed_size =
            (static_cast<uint64_t>(
                 header.PackSizeHigh)
             << 32) |
            header.PackSize;

        const int is_directory =
            (header.Flags & RHDF_DIRECTORY) != 0;

        callback(
            name.c_str(),
            size,
            compressed_size,
            is_directory,
            context);

        code = RARProcessFile(
            archive,
            RAR_SKIP,
            nullptr,
            nullptr);

        if (code != ERAR_SUCCESS)
        {
            result = code;
            break;
        }
    }

    RARCloseArchive(archive);

    return result;
}