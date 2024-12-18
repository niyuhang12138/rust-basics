#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

extern "C" {

const char *hell_bad(const char *name);

void free_str(char *s);

}  // extern "C"
