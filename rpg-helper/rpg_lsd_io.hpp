#include <cstdlib>
#include <string>
#include <vector>

#ifndef RPG_LSD_IO_HPP_INCLUDED
#define RPG_LSD_IO_HPP_INCLUDED

extern "C" {
    int read_rpg_var_lgs(const char *filename, int32_t offset, int32_t count, int32_t *ret);
    int write_rpg_var_lgs(const char *in_filename, const char *out_filename, int32_t offset,
                           int32_t count, const int32_t *variables);

    int read_rpg_switch_lgs(const char *filename, int32_t offset, int32_t count,
                             int8_t *ret);
    int write_rpg_switch_lgs(const char *in_filename, const char *out_filename, int32_t offset,
                              int32_t count, const int8_t *switches);

    int read_rpg_var(const char *filename, int32_t offset, int32_t count, int32_t *ret);
    int write_rpg_var(const char *in_filename, const char *out_filename, int32_t offset,
                       int32_t count, const int32_t *variables);

    int read_rpg_switch(const char *filename, int32_t offset, int32_t count,
                         int8_t *ret);
    int write_rpg_switch(const char *in_filename, const char *out_filename,
                          int32_t offset, int32_t count, const int8_t *switches);
}


#endif
