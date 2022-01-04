#if defined(_MSC_VER) && defined(_WIN32)
const char *lwip_strerr(signed char err) {
    return "";
}
#endif