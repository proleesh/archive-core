#include "../include/sevenzip_bridge.h"
#include "../vendor/7zip/CPP/Common/MyInitGuid.h"
#include "../vendor/7zip/CPP/7zip/ICoder.h"
#include "../vendor/7zip/CPP/7zip/IPassword.h"
#include "../vendor/7zip/CPP/7zip/Compress/DeflateRegister.cpp"

#include "../vendor/7zip/CPP/Windows/PropVariant.h"
#include <filesystem>
#include <string>
#include <vector>
#include <cstdio>

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