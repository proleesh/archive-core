#include "../include/unrar_bridge.h"
#include "unrar/dll.hpp"

#include <cstdint>
#include <cstring>
#include <string>
#include <filesystem>
#include <cstdio>

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

extern "C" int32_t arkive_rar_list_with_password(
    const char *path,
    const char *password,
    arkive_rar_entry_callback callback,
    void *context)
{
    if (
        path == nullptr ||
        password == nullptr ||
        callback == nullptr)
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

    RARSetPassword(
        archive,
        const_cast<char *>(password));

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
static bool is_safe_archive_path(
    const std::string &name)
{
    if (name.empty())
    {
        return false;
    }
    std::filesystem::path path(name);

    // /etc/passwd 같은 절대 경로 차단
    if (path.is_absolute())
    {
        return false;
    }
    // Windows 계열 경로도 방어
    if (name.size() >= 2 &&
        ((name[0] >= 'A' && name[0] <= 'Z') ||
         (name[0] >= 'a' && name[0] <= 'z')) &&
        name[1] == ':')
    {
        return false;
    }

    for (const auto &component : path)
    {
        if (component == "..")
        {
            return false;
        }
    }
    return true;
}
extern "C" int32_t arkive_rar_extract(
    const char *source,
    const char *destination)
{
    if (source == nullptr || destination == nullptr)
    {
        return ERAR_BAD_DATA;
    }
    //

    // Phase 1

    // 모든 entry의 경로를 먼저 검사한다.

    //

    {

        RAROpenArchiveDataEx open_data{};

        open_data.ArcName =

            const_cast<char *>(source);

        open_data.OpenMode = RAR_OM_LIST;

        HANDLE archive =

            RAROpenArchiveEx(&open_data);

        if (archive == nullptr)
        {

            return static_cast<int32_t>(

                open_data.OpenResult

            );
        }

        for (;;)
        {

            RARHeaderDataEx header{};

            int code =

                RARReadHeaderEx(

                    archive,

                    &header

                );

            if (code == ERAR_END_ARCHIVE)
            {

                break;
            }

            if (code != ERAR_SUCCESS)
            {

                RARCloseArchive(archive);

                return code;
            }

            std::string name;

            if (header.FileNameW[0] != L'\0')
            {

                name = wide_to_utf8(

                    header.FileNameW

                );
            }
            else
            {

                name = header.FileName;
            }

            if (!is_safe_archive_path(name))
            {

                RARCloseArchive(archive);

                // Arkive 자체 bridge error

                return 1001;
            }

            // Symlink, hardlink, junction 등의 redirection entry 차단.
            // 첫 버전 Arkive에서는 실제 파일/디렉터리만 추출한다.
            if (header.RedirType != 0)
            {
                RARCloseArchive(archive);
                return 1002;
            }

            code = RARProcessFile(

                archive,

                RAR_SKIP,

                nullptr,

                nullptr

            );

            if (code != ERAR_SUCCESS)
            {

                RARCloseArchive(archive);

                return code;
            }
        }

        RARCloseArchive(archive);
    }

    //

    // Phase 2

    // 검사에 통과했으므로 실제 extraction

    //

    RAROpenArchiveDataEx open_data{};

    open_data.ArcName =

        const_cast<char *>(source);

    open_data.OpenMode = RAR_OM_EXTRACT;

    HANDLE archive =

        RAROpenArchiveEx(&open_data);

    if (archive == nullptr)
    {

        return static_cast<int32_t>(

            open_data.OpenResult

        );
    }

    int32_t result = ERAR_SUCCESS;

    for (;;)
    {

        RARHeaderDataEx header{};

        int code =

            RARReadHeaderEx(

                archive,

                &header

            );

        if (code == ERAR_END_ARCHIVE)
        {

            break;
        }

        if (code != ERAR_SUCCESS)
        {

            result = code;

            break;
        }

        code = RARProcessFile(

            archive,

            RAR_EXTRACT,

            const_cast<char *>(destination),

            nullptr

        );

        if (code != ERAR_SUCCESS)
        {

            result = code;

            break;
        }
    }

    RARCloseArchive(archive);

    return result;
}
extern "C" int32_t arkive_rar_extract_with_password(
    const char *source,
    const char *destination,
    const char *password)
{
    if (
        source == nullptr ||
        destination == nullptr ||
        password == nullptr)
    {
        return ERAR_BAD_DATA;
    }

    RAROpenArchiveDataEx open_data{};

    open_data.ArcName =
        const_cast<char *>(source);

    open_data.OpenMode = RAR_OM_EXTRACT;

    HANDLE archive =
        RAROpenArchiveEx(&open_data);

    fprintf(
        stderr,
        "[Arkive C++] OPEN: handle=%p OpenResult=%u\n",
        archive,
        open_data.OpenResult);

    if (archive == nullptr)
    {
        return static_cast<int32_t>(
            open_data.OpenResult);
    }

    RARSetPassword(
        archive,
        const_cast<char *>(password));

    int32_t result = ERAR_SUCCESS;

    for (;;)
    {
        RARHeaderDataEx header{};

        int code =
            RARReadHeaderEx(
                archive,
                &header);

        fprintf(
            stderr,
            "[Arkive C++] READ HEADER: %d\n",
            code);

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

        // 기존 path traversal 방어
        if (!is_safe_archive_path(name))
        {
            result = 1001;
            break;
        }

        // symlink / hardlink / redirection 차단
        if (header.RedirType != 0)
        {
            result = 1002;
            break;
        }

        code = RARProcessFile(
            archive,
            RAR_EXTRACT,
            const_cast<char *>(destination),
            nullptr);
        fprintf(
            stderr,
            "[Arkive C++] PROCESS FILE: %d\n",
            code);

        if (code != ERAR_SUCCESS)
        {
            result = code;
            break;
        }
    }

    RARCloseArchive(archive);
    fprintf(
        stderr,
        "[Arkive C++] PROCESS FILE: %d\n",
        result);
    return result;
}