/* config.h.  Manually generated for MinGW-w64 (rkflash Windows build).  */

/* Define to the attribute for default visibility. */
#define DEFAULT_VISIBILITY /**/

/* Define to 1 to enable message logging. */
#define ENABLE_LOGGING 1

/* Define to 1 if compiling for a Windows platform. */
#define PLATFORM_WINDOWS 1

/* Define to the attribute for enabling parameter checks on printf-like
   functions. */
#define PRINTF_FORMAT(a, b) __attribute__ ((__format__ (__printf__, a, b)))

/* Define to 1 if you have the <sys/time.h> header file. */
#define HAVE_SYS_TIME_H 1

/* Target Windows 7 and later. */
#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0601
#endif
