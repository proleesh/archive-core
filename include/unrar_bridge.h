#ifndef ARKIVE_UNRAR_BRIDGE_H
#define ARKIVE_UNRAR_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

    typedef void (*arkive_rar_entry_callback)(
        const char *name,
        uint64_t size,
        uint64_t compressed_size,
        int is_directory,
        void *context);

    int32_t arkive_rar_list(
        const char *path,
        arkive_rar_entry_callback callback,
        void *context);

    int32_t arkive_rar_extract(
        const char *source,
        const char *destination);

#ifdef __cplusplus
}
#endif

#endif