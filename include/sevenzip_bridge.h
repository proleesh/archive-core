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

#ifdef __cplusplus
}
#endif