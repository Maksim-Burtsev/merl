#define local static
#define ZEXTERN extern
#ifdef ZTEST
#  define zlocal
#else
#  define zlocal static
#endif
