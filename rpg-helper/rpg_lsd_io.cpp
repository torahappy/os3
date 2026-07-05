/*********************************************************************
 *  rpg_lsd_io.cpp
 *
 *  Provides the following C exported functions:
 *      read_rpg_var      – read 32‑bit variables from a .lsd file
 *      write_rpg_var     – write 32‑bit variables to a .lsd file
 *      read_rpg_switch   – read 8‑bit switches from a .lsd file
 *      write_rpg_switch  – write 8‑bit switches to a .lsd file
 *
 *  The implementation is intentionally lightweight – it only
 *  touches the parts of the save file that contain the variables
 *  and switches.  All other data is left untouched.
 *********************************************************************/

#include <cstdint>     // int8_t, int32_t, etc.
#include <cstring>    // memcpy
#include <string_view>
#include <memory>
#include <iostream>

// ---------------  liblcf headers --------------------------------
#include "lcf/lsd/reader.h"
#include "lcf/rpg/save.h"
#include "lcf/rpg/savesystem.h"
#include "lcf/engine_version.h"   // defines lcf::EngineVersion

// ---------------  C interface -----------------------------------
extern "C" {

    /*  -----------------------------------------------------------------
     *  read_rpg_var
     *
     *  Parameters
     *    filename : path to the .lsd file
     *    offset   : first variable index to read (0‑based)
     *    count    : how many variables to read
     *    ret      : caller‑supplied buffer; must be at least count*4 bytes
     *
     *  Returns  0 on success, -1 on error.
     ----------------------------------------------------------------- */
    int read_rpg_var(const char* filename,
                      int32_t offset,
                      int32_t count,
                      int32_t* ret)
    {
        if (!filename || !ret || count <= 0 || offset < 0) return -1;

        // Load the save file
        std::unique_ptr<lcf::rpg::Save> savePtr =
            lcf::LSD_Reader::Load(std::string_view(filename));
        if (!savePtr) return -1;                      // file not found / parse error

        const std::vector<int32_t>& vars = savePtr->system.variables;
        if (static_cast<size_t>(offset + count) > vars.size()) return -1;

        // Copy the variables
        std::memcpy(ret, vars.data() + offset, static_cast<size_t>(count) * sizeof(int32_t));
        return 0;
    }

    /*  -----------------------------------------------------------------
     *  write_rpg_var
     *
     *  Parameters
     *    in_filename   : original .lsd file
     *    out_filename  : destination .lsd file (may overwrite)
     *    offset        : first variable index to write
     *    count         : how many variables to write
     *    variables     : caller‑supplied array of new values
     *
     *  Returns  0 on success, -1 on error.
     ----------------------------------------------------------------- */
    int write_rpg_var(const char* in_filename,
                       const char* out_filename,
                       int32_t offset,
                       int32_t count,
                       const int32_t* variables)
    {
        if (!in_filename || !out_filename || !variables ||
            count <= 0 || offset < 0) return -1;

        // Load the original file
        std::unique_ptr<lcf::rpg::Save> savePtr =
            lcf::LSD_Reader::Load(std::string_view(in_filename));
        if (!savePtr) return -1;

        std::vector<int32_t>& vars = savePtr->system.variables;
        if (static_cast<size_t>(offset + count) > vars.size()) return -1;

        // Copy the new values
        std::memcpy(vars.data() + offset, variables, static_cast<size_t>(count) * sizeof(int32_t));

        // Write the modified file
        if (!lcf::LSD_Reader::Save(std::string_view(out_filename),
                                   *savePtr,
                                   lcf::EngineVersion::EasyRPG,   // choose a valid engine
                                   "")) return -1;                // default encoding

        return 0;
    }

    /*  -----------------------------------------------------------------
     *  read_rpg_switch
     *
     *  Parameters
     *    filename : path to the .lsd file
     *    offset   : first switch index to read (0‑based)
     *    count    : how many switches to read
     *    ret      : caller‑supplied buffer; must be at least count bytes
     *
     *  Returns  0 on success, -1 on error.
     ----------------------------------------------------------------- */
    int read_rpg_switch(const char* filename,
                        int32_t offset,
                        int32_t count,
                        int8_t* ret)
    {
        if (!filename || !ret || count <= 0 || offset < 0) return -1;

        std::unique_ptr<lcf::rpg::Save> savePtr =
            lcf::LSD_Reader::Load(std::string_view(filename));
        if (!savePtr) return -1;

        const std::vector<bool>& sw = savePtr->system.switches;
        if (static_cast<size_t>(offset + count) > sw.size()) return -1;

        for (int32_t i = 0; i < count; ++i)
            ret[i] = sw[static_cast<size_t>(offset) + i] ? 1 : 0;

        return 0;
    }

    /*  -----------------------------------------------------------------
     *  write_rpg_switch
     *
     *  Parameters
     *    in_filename  : original .lsd file
     *    out_filename : destination .lsd file
     *    offset       : first switch index to write
     *    count        : how many switches to write
     *    switches     : caller‑supplied array of new values (0/1)
     *
     *  Returns  0 on success, -1 on error.
     ----------------------------------------------------------------- */
    int write_rpg_switch(const char* in_filename,
                         const char* out_filename,
                         int32_t offset,
                         int32_t count,
                         const int8_t* switches)
    {
        if (!in_filename || !out_filename || !switches ||
            count <= 0 || offset < 0) return -1;

        std::unique_ptr<lcf::rpg::Save> savePtr =
            lcf::LSD_Reader::Load(std::string_view(in_filename));
        if (!savePtr) return -1;

        std::vector<bool>& sw = savePtr->system.switches;
        if (static_cast<size_t>(offset + count) > sw.size()) return -1;

        for (int32_t i = 0; i < count; ++i)
            sw[static_cast<size_t>(offset) + i] = (switches[i] != 0);

        if (!lcf::LSD_Reader::Save(std::string_view(out_filename),
                                   *savePtr,
                                   lcf::EngineVersion::EasyRPG,
                                   "")) return -1;

        return 0;
    }
}

/*********************************************************************
 *  End of rpg_lsd_io.cpp
 *********************************************************************/
