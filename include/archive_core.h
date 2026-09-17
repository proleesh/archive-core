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

    char *archive_list(const char *path);

#ifdef __cplusplus
}
#endif

#endif