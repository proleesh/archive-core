#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

    // return:
    //   0 = success
    //   1 = invalid argument
    //   5 = I/O error
    //   6 = internal error
    //   100 = 7-Zip compression error
    int32_t arkive_7zip_create_zip(
        const char *const *sources,
        uint32_t source_count,
        const char *destination,
        int32_t compression_level,
        uint32_t thread_count);

    int32_t arkive_7zip_create_7z(
        const char *const *sources,
        uint32_t source_count,
        const char *destination,
        int32_t compression_level,
        uint32_t thread_count);

    int32_t arkive_7zip_list(
        const char *archive_path,
        char **json_out);

    int32_t arkive_7zip_extract(
        const char *archive_path,
        const char *destination);

    void arkive_7zip_string_free(
        char *value);

#ifdef __cplusplus
}
#endif