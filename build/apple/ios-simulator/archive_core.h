#ifndef ARCHIVE_CORE_H
#define ARCHIVE_CORE_H
#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

    char *archive_detect_format(const char *path);

    void archive_string_free(char *value);

    int32_t archive_extract(

        const char *source,

        const char *destination

    );

    int32_t archive_extract_with_password(
        const char *source,
        const char *destination,
        const char *password);

    int32_t archive_list_v2(
        const char *path,
        char **json_out);

    int32_t archive_list_with_password_v2(
        const char *path,
        const char *password,
        char **json_out);

    int32_t archive_create_zip(
        const char *sources_json,
        const char *destination,
        int32_t performance);

    int32_t archive_create_7z(
        const char *sources_json,
        const char *destination,
        int32_t compression_level,
        uint32_t thread_count);

    char *archive_list(const char *path);

    char *archive_list_with_password(
        const char *path,
        const char *password);

#ifdef __cplusplus
}
#endif

#endif