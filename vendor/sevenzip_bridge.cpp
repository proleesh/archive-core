#include "../include/sevenzip_bridge.h"
#include "../vendor/7zip/CPP/Common/MyInitGuid.h"
#include "../vendor/7zip/CPP/7zip/ICoder.h"
#include "../vendor/7zip/CPP/7zip/IPassword.h"
#include "../vendor/7zip/CPP/7zip/Compress/DeflateRegister.cpp"
#include "../vendor/7zip/CPP/7zip/Compress/LzmaRegister.cpp"
#include "../vendor/7zip/CPP/7zip/Compress/Lzma2Register.cpp"

#include "../vendor/7zip/CPP/Common/MyLinux.h"
#include "../vendor/7zip/CPP/Windows/PropVariant.h"
#include "../vendor/7zip/CPP/7zip/Archive/7z/7zHandler.h"
#include <filesystem>
#include <string>
#include <vector>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <sstream>

#include "../vendor/7zip/CPP/7zip/Common/FileStreams.h"
#include "../vendor/7zip/CPP/7zip/Archive/IArchive.h"
#include "../vendor/7zip/CPP/7zip/Archive/Zip/ZipHandler.h"

using namespace NWindows;
using namespace NCOM;

namespace
{

    struct ArkiveZipItem
    {
        std::filesystem::path diskPath;
        std::wstring archivePath;
        UInt64 size = 0;
        bool isDirectory = false;
    };

    static std::wstring toArchivePath(
        const std::filesystem::path &path)
    {
        std::wstring result = path.generic_wstring();

        while (!result.empty() && result.front() == L'/')
        {
            result.erase(result.begin());
        }

        return result;
    }

    static void collectItem(
        const std::filesystem::path &diskPath,
        const std::filesystem::path &archivePath,
        std::vector<ArkiveZipItem> &items)
    {
        std::error_code ec;

        const bool isDirectory =
            std::filesystem::is_directory(diskPath, ec);

        if (ec)
        {
            return;
        }

        ArkiveZipItem item;
        item.diskPath = diskPath;
        item.archivePath = toArchivePath(archivePath);
        item.isDirectory = isDirectory;

        if (!isDirectory)
        {
            item.size =
                std::filesystem::file_size(diskPath, ec);

            if (ec)
            {
                item.size = 0;
            }
        }

        items.push_back(std::move(item));

        if (!isDirectory)
        {
            return;
        }

        for (const auto &entry :
             std::filesystem::directory_iterator(diskPath, ec))
        {
            if (ec)
            {
                break;
            }

            collectItem(
                entry.path(),
                archivePath / entry.path().filename(),
                items);
        }
    }

    static bool collectSources(
        const char *const *sources,
        uint32_t sourceCount,
        std::vector<ArkiveZipItem> &items)
    {
        for (uint32_t i = 0; i < sourceCount; ++i)
        {
            if (sources[i] == nullptr || sources[i][0] == '\0')
            {
                return false;
            }

            std::filesystem::path source =
                std::filesystem::u8path(sources[i]);

            std::error_code ec;

            if (!std::filesystem::exists(source, ec) || ec)
            {
                return false;
            }

            const auto filename = source.filename();

            if (filename.empty())
            {
                return false;
            }

            collectItem(
                source,
                filename,
                items);
        }

        return !items.empty();
    }
    static std::string arkiveJsonEscape(
        const std::string &value)
    {
        std::string result;
        result.reserve(value.size() + 16);

        for (unsigned char ch : value)
        {
            switch (ch)
            {
            case '"':
                result += "\\\"";
                break;

            case '\\':
                result += "\\\\";
                break;

            case '\b':
                result += "\\b";
                break;

            case '\f':
                result += "\\f";
                break;

            case '\n':
                result += "\\n";
                break;

            case '\r':
                result += "\\r";
                break;

            case '\t':
                result += "\\t";
                break;

            default:
                if (ch < 0x20)
                {
                    char buffer[7];

                    std::snprintf(
                        buffer,
                        sizeof(buffer),
                        "\\u%04x",
                        static_cast<unsigned int>(ch));

                    result += buffer;
                }
                else
                {
                    result += static_cast<char>(ch);
                }

                break;
            }
        }

        return result;
    }
    static std::string arkiveWideToUtf8(
        const std::wstring &value)
    {
        std::string result;

        for (wchar_t wc : value)
        {
            uint32_t codePoint =
                static_cast<uint32_t>(wc);

            if (codePoint <= 0x7F)
            {
                result.push_back(
                    static_cast<char>(codePoint));
            }
            else if (codePoint <= 0x7FF)
            {
                result.push_back(
                    static_cast<char>(
                        0xC0 | (codePoint >> 6)));

                result.push_back(
                    static_cast<char>(
                        0x80 | (codePoint & 0x3F)));
            }
            else if (codePoint <= 0xFFFF)
            {
                result.push_back(
                    static_cast<char>(
                        0xE0 | (codePoint >> 12)));

                result.push_back(
                    static_cast<char>(
                        0x80 | ((codePoint >> 6) & 0x3F)));

                result.push_back(
                    static_cast<char>(
                        0x80 | (codePoint & 0x3F)));
            }
            else
            {
                result.push_back(
                    static_cast<char>(
                        0xF0 | (codePoint >> 18)));

                result.push_back(
                    static_cast<char>(
                        0x80 | ((codePoint >> 12) & 0x3F)));

                result.push_back(
                    static_cast<char>(
                        0x80 | ((codePoint >> 6) & 0x3F)));

                result.push_back(
                    static_cast<char>(
                        0x80 | (codePoint & 0x3F)));
            }
        }

        return result;
    }

    static bool arkiveIsSafeArchivePath(
        const std::filesystem::path &path)
    {
        if (path.empty())
            return false;

        if (path.is_absolute())
            return false;

        if (path.has_root_name() ||
            path.has_root_directory())
        {
            return false;
        }

        for (const auto &component : path)
        {
            if (component == "..")
                return false;
        }

        return true;
    }
    static bool arkiveHasSymlinkAncestor(
        const std::filesystem::path &destinationRoot,
        const std::filesystem::path &relativePath)
    {
        std::error_code ec;

        std::filesystem::path current = destinationRoot;

        // destination root 자체가 기존 symlink인 경우도 거부
        const auto rootStatus =
            std::filesystem::symlink_status(current, ec);

        if (ec)
        {
            // 아직 존재하지 않는 root는 허용
            if (ec != std::errc::no_such_file_or_directory)
                return true;

            ec.clear();
        }
        else if (std::filesystem::is_symlink(rootStatus))
        {
            return true;
        }

        // 마지막 component(실제 생성할 파일)는 제외하고
        // 부모 경로들만 검사
        const auto parent = relativePath.parent_path();

        for (const auto &component : parent)
        {
            current /= component;

            const auto status =
                std::filesystem::symlink_status(current, ec);

            if (ec)
            {
                if (ec == std::errc::no_such_file_or_directory)
                {
                    ec.clear();
                    continue;
                }

                return true;
            }

            if (std::filesystem::is_symlink(status))
                return true;
        }

        return false;
    }
    class ArkiveArchiveUpdateCallback Z7_final : public IArchiveUpdateCallback,
                                                 public CMyUnknownImp
    {
        Z7_IFACES_IMP_UNK_1(IArchiveUpdateCallback)
        Z7_IFACE_COM7_IMP(IProgress)

    public:
        const std::vector<ArkiveZipItem> *items = nullptr;

        explicit ArkiveArchiveUpdateCallback(
            const std::vector<ArkiveZipItem> *sourceItems)
            : items(sourceItems)
        {
        }
    };
    class ArkiveArchiveExtractCallback Z7_final : public IArchiveExtractCallback,
                                                  public CMyUnknownImp
    {
        Z7_IFACES_IMP_UNK_1(IArchiveExtractCallback)
        Z7_IFACE_COM7_IMP(IProgress)

    private:
        IInArchive *archive_ = nullptr;
        std::filesystem::path destination_;

        CMyComPtr<ISequentialOutStream> currentStream_;

        bool unsafePath_ = false;
        bool ioError_ = false;
        bool archiveError_ = false;
        bool unsafeRedirection_ = false;

    public:
        ArkiveArchiveExtractCallback(
            IInArchive *archive,
            const std::filesystem::path &destination)
            : archive_(archive),
              destination_(destination)
        {
        }

        bool HasUnsafePath() const
        {
            return unsafePath_;
        }
        bool HasUnsafeRedirection() const
        {
            return unsafeRedirection_;
        }

        bool HasIoError() const
        {
            return ioError_;
        }

        bool HasArchiveError() const
        {
            return archiveError_;
        }
    };
    Z7_COM7F_IMF(
        ArkiveArchiveExtractCallback::SetTotal(
            UInt64 /* size */))
    {
        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveExtractCallback::SetCompleted(
            const UInt64 * /* completeValue */))
    {
        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveExtractCallback::GetStream(
            UInt32 index,
            ISequentialOutStream **outStream,
            Int32 askExtractMode))
    {
        if (!outStream)
            return E_INVALIDARG;

        *outStream = nullptr;
        currentStream_.Release();

        if (askExtractMode !=
            NArchive::NExtract::NAskMode::kExtract)
        {
            return S_OK;
        }

        NCOM::CPropVariant pathProperty;

        HRESULT result =
            archive_->GetProperty(
                index,
                kpidPath,
                &pathProperty);

        if (result != S_OK ||
            pathProperty.vt != VT_BSTR ||
            pathProperty.bstrVal == nullptr)
        {
            archiveError_ = true;
            return E_FAIL;
        }

        const std::wstring widePath(
            pathProperty.bstrVal,
            SysStringLen(pathProperty.bstrVal));

        const std::string utf8Path =
            arkiveWideToUtf8(widePath);

        const std::filesystem::path relativePath =
            std::filesystem::u8path(utf8Path);

        if (!arkiveIsSafeArchivePath(relativePath))
        {
            std::fprintf(
                stderr,
                "[Arkive] Unsafe 7Z path rejected: %s\n",
                utf8Path.c_str());

            unsafePath_ = true;
            return E_FAIL;
        }
        NCOM::CPropVariant symLinkProperty;
        NCOM::CPropVariant hardLinkProperty;

        result = archive_->GetProperty(
            index,
            kpidSymLink,
            &symLinkProperty);

        if (result != S_OK)
        {
            archiveError_ = true;
            return E_FAIL;
        }

        result = archive_->GetProperty(
            index,
            kpidHardLink,
            &hardLinkProperty);

        if (result != S_OK)
        {
            archiveError_ = true;
            return E_FAIL;
        }

        const bool hasSymLink =
            symLinkProperty.vt == VT_BSTR &&
            symLinkProperty.bstrVal != nullptr &&
            SysStringLen(symLinkProperty.bstrVal) > 0;

        const bool hasHardLink =
            hardLinkProperty.vt == VT_BSTR &&
            hardLinkProperty.bstrVal != nullptr &&
            SysStringLen(hardLinkProperty.bstrVal) > 0;
        NCOM::CPropVariant attribProperty;

        result = archive_->GetProperty(
            index,
            kpidAttrib,
            &attribProperty);

        if (result != S_OK)
        {
            archiveError_ = true;
            return E_FAIL;
        }

        bool hasUnixSymLink = false;

        if (attribProperty.vt == VT_UI4)
        {
            const UInt32 attrib = attribProperty.ulVal;

            hasUnixSymLink = MY_LIN_S_ISLNK(attrib >> 16);
        }
        else if (attribProperty.vt != VT_EMPTY)
        {
            archiveError_ = true;
            return E_FAIL;
        }
        if (hasSymLink || hasHardLink || hasUnixSymLink)
        {
            std::fprintf(
                stderr,
                "[Arkive] 7Z link entry rejected: %s\n",
                utf8Path.c_str());

            unsafeRedirection_ = true;

            return E_FAIL;
        }

        NCOM::CPropVariant directoryProperty;

        result =
            archive_->GetProperty(
                index,
                kpidIsDir,
                &directoryProperty);

        if (result != S_OK)
        {
            archiveError_ = true;
            return E_FAIL;
        }

        const bool isDirectory =
            directoryProperty.vt == VT_BOOL &&
            directoryProperty.boolVal != VARIANT_FALSE;

        const std::filesystem::path outputPath =
            destination_ / relativePath;

        if (arkiveHasSymlinkAncestor(
                destination_,
                relativePath))
        {
            std::fprintf(
                stderr,
                "[Arkive] Symlink ancestor rejected: %s\n",
                utf8Path.c_str());

            unsafeRedirection_ = true;
            return E_FAIL;
        }
        std::error_code ec;

        if (isDirectory)
        {
            std::filesystem::create_directories(
                outputPath,
                ec);

            if (ec)
            {
                ioError_ = true;
                return E_FAIL;
            }

            return S_OK;
        }

        const auto parent =
            outputPath.parent_path();

        if (!parent.empty())
        {
            std::filesystem::create_directories(
                parent,
                ec);

            if (ec)
            {
                ioError_ = true;
                return E_FAIL;
            }
        }

        COutFileStream *fileStreamSpec =
            new COutFileStream;

        CMyComPtr<ISequentialOutStream> fileStream =
            fileStreamSpec;

        const std::string outputNative =
            outputPath.string();

        if (!fileStreamSpec->Create_ALWAYS(
                outputNative.c_str()))
        {
            std::fprintf(
                stderr,
                "[Arkive] Failed to create extracted file: %s\n",
                outputNative.c_str());

            ioError_ = true;
            return E_FAIL;
        }

        currentStream_ = fileStream;

        *outStream = fileStream.Detach();

        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveExtractCallback::PrepareOperation(
            Int32 /* askExtractMode */))
    {
        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveExtractCallback::SetOperationResult(
            Int32 opRes))
    {
        currentStream_.Release();

        if (opRes !=
            NArchive::NExtract::NOperationResult::kOK)
        {
            std::fprintf(
                stderr,
                "[Arkive] 7Z extraction operation failed: %d\n",
                opRes);

            archiveError_ = true;
        }

        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveUpdateCallback::SetTotal(UInt64 /* size */))
    {
        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveUpdateCallback::SetCompleted(
            const UInt64 * /* completeValue */))
    {
        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveUpdateCallback::GetUpdateItemInfo(
            UInt32 /* index */,
            Int32 *newData,
            Int32 *newProperties,
            UInt32 *indexInArchive))
    {
        std::fprintf(
            stderr,
            "[Arkive] GetUpdateItemInfo\n");
        if (newData)
            *newData = BoolToInt(true);

        if (newProperties)
            *newProperties = BoolToInt(true);

        if (indexInArchive)
            *indexInArchive = static_cast<UInt32>(-1);

        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveUpdateCallback::GetProperty(
            UInt32 index,
            PROPID propID,
            PROPVARIANT *value))
    {
        std::fprintf(
            stderr,
            "[Arkive] GetProperty index=%u propID=%u\n",
            static_cast<unsigned>(index),
            static_cast<unsigned>(propID));
        if (!items || index >= items->size() || !value)
            return E_INVALIDARG;

        const ArkiveZipItem &item = (*items)[index];

        NCOM::CPropVariant prop;

        switch (propID)
        {
        case kpidPath:
            prop = item.archivePath.c_str();
            break;

        case kpidIsDir:
            prop = item.isDirectory;
            break;

        case kpidSize:
            prop = item.size;
            break;

        case kpidIsAnti:
            prop = false;
            break;

        default:
            break;
        }

        prop.Detach(value);

        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveUpdateCallback::GetStream(
            UInt32 index,
            ISequentialInStream **inStream))
    {
        std::fprintf(
            stderr,
            "[Arkive] GetStream index=%u\n",
            static_cast<unsigned>(index));
        if (!inStream)
            return E_INVALIDARG;

        *inStream = nullptr;

        if (!items || index >= items->size())
            return E_INVALIDARG;

        const ArkiveZipItem &item = (*items)[index];

        // Directory entry에는 input stream이 필요 없습니다.
        if (item.isDirectory)
            return S_OK;

        CInFileStream *streamSpec = new CInFileStream;

        CMyComPtr<ISequentialInStream> stream(streamSpec);

        const std::string diskPath =
            item.diskPath.string();

        if (!streamSpec->Open(diskPath.c_str()))
        {
            std::fprintf(
                stderr,
                "[Arkive] GetStream Open failed: %s\n",
                diskPath.c_str());
            return S_FALSE;
        }

        *inStream = stream.Detach();

        return S_OK;
    }

    Z7_COM7F_IMF(
        ArkiveArchiveUpdateCallback::SetOperationResult(
            Int32 /* operationResult */))
    {
        return S_OK;
    }
}

extern "C" int32_t arkive_7zip_create_zip(
    const char *const *sources,
    uint32_t source_count,
    const char *destination,
    int32_t compression_level,
    uint32_t thread_count)
{
    if (
        sources == nullptr ||
        source_count == 0 ||
        destination == nullptr ||
        destination[0] == '\0')
    {
        return 1;
    }

    if (compression_level < 0)
    {
        compression_level = 0;
    }

    if (compression_level > 9)
    {
        compression_level = 9;
    }

    if (thread_count == 0)
    {
        thread_count = 1;
    }

    NArchive::NZip::CHandler *handlerSpec =
        new NArchive::NZip::CHandler();

    CMyComPtr<IOutArchive> archive = handlerSpec;

    CMyComPtr<ISetProperties> setProperties;

    HRESULT result = archive->QueryInterface(
        IID_ISetProperties,
        reinterpret_cast<void **>(&setProperties));

    if (result != S_OK || !setProperties)
    {
        return 100;
    }

    const wchar_t *propertyNames[] = {
        L"x",
        L"mt"};

    CPropVariant propertyValues[2];

    propertyValues[0] = static_cast<UInt32>(
        compression_level);

    propertyValues[1] = static_cast<UInt32>(
        thread_count);

    result = setProperties->SetProperties(
        propertyNames,
        propertyValues,
        2);

    if (result != S_OK)
    {
        std::fprintf(
            stderr,
            "[Arkive] SetProperties failed: HRESULT=0x%08X\n",
            static_cast<unsigned int>(result));

        return 100;
    }

    /*
     * 다음 단계에서:
     *
     * 1. sources[]를 실제 filesystem entry 목록으로 변환
     * 2. IArchiveUpdateCallback 구현
     * 3. CInFileStream 제공
     * 4. COutFileStream 생성
     * 5. archive->UpdateItems(...)
     *
     * 을 연결한다.
     */

    std::vector<ArkiveZipItem> items;

    if (!collectSources(
            sources,
            source_count,
            items))
    {
        return 1;
    }

    COutFileStream *outStreamSpec =
        new COutFileStream;

    CMyComPtr<IOutStream> outStream(
        outStreamSpec);

    const std::filesystem::path destinationPath =
        std::filesystem::u8path(destination);

    const std::string destinationNative =
        destinationPath.string();

    if (!outStreamSpec->Create_ALWAYS(
            destinationNative.c_str()))
    {
        std::fprintf(
            stderr,
            "[Arkive] Failed to create destination: %s\n",
            destinationNative.c_str());

        return 5;
    }

    ArkiveArchiveUpdateCallback *callbackSpec =
        new ArkiveArchiveUpdateCallback(&items);

    CMyComPtr<IArchiveUpdateCallback> callback(
        callbackSpec);

    result = archive->UpdateItems(
        outStream,
        static_cast<UInt32>(items.size()),
        callback);

    if (result != S_OK)
    {
        std::fprintf(
            stderr,
            "[Arkive] UpdateItems failed: HRESULT=0x%08X\n",
            static_cast<unsigned int>(result));

        outStreamSpec->Close();
        return 100;
    }

    const HRESULT closeResult =
        outStreamSpec->Close();

    if (closeResult != S_OK)
    {
        return 5;
    }

    return 0;
}
extern "C" int32_t arkive_7zip_create_7z(
    const char *const *sources,
    uint32_t source_count,
    const char *destination,
    int32_t compression_level,
    uint32_t thread_count)
{
    if (
        sources == nullptr ||
        source_count == 0 ||
        destination == nullptr ||
        destination[0] == '\0')
    {
        return 1;
    }

    if (compression_level < 0)
        compression_level = 0;

    if (compression_level > 9)
        compression_level = 9;

    if (thread_count == 0)
        thread_count = 1;

    std::vector<ArkiveZipItem> items;

    if (!collectSources(
            sources,
            source_count,
            items))
    {
        return 1;
    }

    // 7Z handler
    NArchive::N7z::CHandler *handlerSpec =
        new NArchive::N7z::CHandler();

    CMyComPtr<IOutArchive> archive =
        handlerSpec;

    CMyComPtr<ISetProperties> setProperties;

    HRESULT result = archive->QueryInterface(
        IID_ISetProperties,
        reinterpret_cast<void **>(&setProperties));

    if (result != S_OK || !setProperties)
        return 100;

    //
    // 7Z compression properties
    //
    const wchar_t *propertyNames[] = {
        L"x",
        L"mt"};

    NCOM::CPropVariant propertyValues[2];

    // Compression level
    propertyValues[0] =
        static_cast<UInt32>(compression_level);

    // Requested worker count
    propertyValues[1] =
        static_cast<UInt32>(thread_count);

    result = setProperties->SetProperties(
        propertyNames,
        propertyValues,
        2);

    if (result != S_OK)
    {
        std::fprintf(
            stderr,
            "[Arkive] 7Z SetProperties failed: HRESULT=0x%08X\n",
            static_cast<unsigned int>(result));
        return 100;
    }

    //
    // Output .7z
    //
    COutFileStream *outStreamSpec =
        new COutFileStream;

    CMyComPtr<IOutStream> outStream(
        outStreamSpec);

    const std::filesystem::path destinationPath =
        std::filesystem::u8path(destination);

    const std::string destinationNative =
        destinationPath.string();

    if (!outStreamSpec->Create_ALWAYS(
            destinationNative.c_str()))
    {
        std::fprintf(
            stderr,
            "[Arkive] Failed to create 7Z destination: %s\n",
            destinationNative.c_str());

        return 5;
    }

    //
    // Reuse Arkive's existing update callback
    //
    ArkiveArchiveUpdateCallback *callbackSpec =
        new ArkiveArchiveUpdateCallback(&items);

    CMyComPtr<IArchiveUpdateCallback> callback =
        callbackSpec;

    result = archive->UpdateItems(
        outStream,
        static_cast<UInt32>(items.size()),
        callback);

    if (result != S_OK)
    {
        std::fprintf(
            stderr,
            "[Arkive] 7Z UpdateItems failed: HRESULT=0x%08X\n",
            static_cast<unsigned int>(result));
        outStreamSpec->Close();
        return 100;
    }

    const HRESULT closeResult =
        outStreamSpec->Close();

    if (closeResult != S_OK)
        return 5;

    return 0;
}
extern "C" int32_t arkive_7zip_list(
    const char *archive_path,
    char **json_out)
{
    if (
        archive_path == nullptr ||
        archive_path[0] == '\0' ||
        json_out == nullptr)
    {
        return 1;
    }

    *json_out = nullptr;

    const std::filesystem::path archivePath =
        std::filesystem::u8path(archive_path);

    const std::string archiveNative =
        archivePath.string();

    //
    // Input stream
    //
    CInFileStream *inStreamSpec =
        new CInFileStream;

    CMyComPtr<IInStream> inStream(
        inStreamSpec);

    if (!inStreamSpec->Open(
            archiveNative.c_str()))
    {
        return 5;
    }

    //
    // 7Z reader
    //
    NArchive::N7z::CHandler *handlerSpec =
        new NArchive::N7z::CHandler();

    CMyComPtr<IInArchive> archive =
        handlerSpec;

    HRESULT result =
        archive->Open(
            inStream,
            nullptr,
            nullptr);

    if (result != S_OK)
    {
        return 3;
    }

    UInt32 itemCount = 0;

    result =
        archive->GetNumberOfItems(
            &itemCount);

    if (result != S_OK)
    {
        archive->Close();
        return 100;
    }

    std::ostringstream json;

    json << "[";

    for (UInt32 index = 0;
         index < itemCount;
         ++index)
    {
        NCOM::CPropVariant pathProperty;
        NCOM::CPropVariant sizeProperty;
        NCOM::CPropVariant packedSizeProperty;
        NCOM::CPropVariant directoryProperty;

        if (archive->GetProperty(
                index,
                kpidPath,
                &pathProperty) != S_OK)
        {
            archive->Close();
            return 100;
        }

        archive->GetProperty(
            index,
            kpidSize,
            &sizeProperty);

        archive->GetProperty(
            index,
            kpidPackSize,
            &packedSizeProperty);

        archive->GetProperty(
            index,
            kpidIsDir,
            &directoryProperty);

        std::string name;

        if (pathProperty.vt == VT_BSTR &&
            pathProperty.bstrVal != nullptr)
        {
            name = arkiveWideToUtf8(
                std::wstring(
                    pathProperty.bstrVal,
                    SysStringLen(
                        pathProperty.bstrVal)));
        }

        UInt64 size = 0;

        if (sizeProperty.vt == VT_UI8)
        {
            size =
                sizeProperty.uhVal.QuadPart;
        }

        UInt64 compressedSize = 0;

        if (packedSizeProperty.vt == VT_UI8)
        {
            compressedSize =
                packedSizeProperty.uhVal.QuadPart;
        }

        bool isDirectory = false;

        if (directoryProperty.vt == VT_BOOL)
        {
            isDirectory =
                directoryProperty.boolVal != VARIANT_FALSE;
        }

        if (index != 0)
        {
            json << ",";
        }

        json
            << "{"
            << "\"name\":\""
            << arkiveJsonEscape(name)
            << "\","
            << "\"size\":"
            << size
            << ","
            << "\"compressed_size\":"
            << compressedSize
            << ","
            << "\"is_directory\":"
            << (isDirectory ? "true" : "false")
            << "}";
    }

    json << "]";

    archive->Close();

    const std::string output =
        json.str();

    char *buffer =
        static_cast<char *>(
            std::malloc(output.size() + 1));

    if (buffer == nullptr)
    {
        return 5;
    }

    std::memcpy(
        buffer,
        output.c_str(),
        output.size() + 1);

    *json_out = buffer;

    return 0;
}

extern "C" void arkive_7zip_string_free(
    char *value)
{
    std::free(value);
}
extern "C" int32_t arkive_7zip_extract(
    const char *archive_path,
    const char *destination)
{
    if (
        archive_path == nullptr ||
        archive_path[0] == '\0' ||
        destination == nullptr ||
        destination[0] == '\0')
    {
        return 1;
    }

    const std::filesystem::path archivePath =
        std::filesystem::u8path(archive_path);

    const std::filesystem::path destinationPath =
        std::filesystem::u8path(destination);

    std::error_code ec;

    std::filesystem::create_directories(
        destinationPath,
        ec);

    if (ec)
        return 5;

    CInFileStream *inStreamSpec =
        new CInFileStream;

    CMyComPtr<IInStream> inStream =
        inStreamSpec;

    const std::string archiveNative =
        archivePath.string();

    if (!inStreamSpec->Open(
            archiveNative.c_str()))
    {
        return 5;
    }

    NArchive::N7z::CHandler *handlerSpec =
        new NArchive::N7z::CHandler();

    CMyComPtr<IInArchive> archive =
        handlerSpec;

    HRESULT result =
        archive->Open(
            inStream,
            nullptr,
            nullptr);

    if (result != S_OK)
        return 3;

    ArkiveArchiveExtractCallback *callbackSpec =
        new ArkiveArchiveExtractCallback(
            archive,
            destinationPath);

    CMyComPtr<IArchiveExtractCallback> callback =
        callbackSpec;

    //
    // nullptr + 0xFFFFFFFF means extract all items.
    //
    result =
        archive->Extract(
            nullptr,
            static_cast<UInt32>(-1),
            0,
            callback);

    archive->Close();

    if (callbackSpec->HasUnsafePath())
        return 1001;

    if (callbackSpec->HasUnsafeRedirection())
    {
        return 1002;
    }

    if (callbackSpec->HasIoError())
        return 5;

    if (callbackSpec->HasArchiveError())
        return 3;

    if (result != S_OK)
    {
        std::fprintf(
            stderr,
            "[Arkive] 7Z Extract failed: HRESULT=0x%08X\n",
            static_cast<unsigned int>(result));

        return 3;
    }

    return 0;
}