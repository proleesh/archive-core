#ifndef ARCHIVE_CORE_H
#define ARCHIVE_CORE_H

#ifdef __cplusplus
extern "C" {
#endif

char *archive_detect_format(const char *path);

void archive_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif