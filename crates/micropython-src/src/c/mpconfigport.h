#include <port/mpconfigport_common.h>

#define MICROPY_CONFIG_ROM_LEVEL                (MICROPY_CONFIG_ROM_LEVEL_EVERYTHING)

#define MICROPY_OBJ_REPR (MICROPY_OBJ_REPR_A) // TODO
#define MP_INT_TYPE (MP_INT_TYPE_INT64) // TODO: does this work?

#define MICROPY_ENABLE_COMPILER                 (1)
#define MICROPY_ENABLE_GC                       (1)
#define MICROPY_PY_GC                           (1)
#define MICROPY_PY_SYS                          (0)