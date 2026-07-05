/*********************************************************************
 *  rpg_lsd_io_test.cpp
 *
 *  Test harness for the four C‑exported functions that were
 *  implemented in *rpg_lsd_io.cpp*.  The test does the following
 *
 *    1.  Builds a minimal .lsd file with known variables / switches.
 *    2.  Calls read_rpg_var / read_rpg_switch to verify that the
 *        original values are returned correctly.
 *    3.  Calls write_rpg_var / write_rpg_switch to change those
 *        values.
 *    4.  Reads them back again to make sure the changes were
 *        written.
 *
 *  The file is written to a temporary location under the current
 *  working directory; the program cleans it up before exiting.
 *********************************************************************/

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <iostream>
#include <fstream>
#include <filesystem>          // C++17
#include <cassert>
extern "C" {
    int read_rpg_var(const char* filename,
                      int32_t offset,
                      int32_t count,
                      int32_t* ret);

    int write_rpg_var(const char* in_filename,
                       const char* out_filename,
                       int32_t offset,
                       int32_t count,
                       const int32_t* variables);

    int read_rpg_switch(const char* filename,
                        int32_t offset,
                        int32_t count,
                        int8_t* ret);

    int write_rpg_switch(const char* in_filename,
                         const char* out_filename,
                         int32_t offset,
                         int32_t count,
                         const int8_t* switches);
}
#include "lcf/rpg/save.h"
#include "lcf/rpg/savesystem.h"
#include "lcf/lsd/reader.h"

extern "C" {
    int read_rpg_var(const char* filename,
                      int32_t offset,
                      int32_t count,
                      int32_t* ret);

    int write_rpg_var(const char* in_filename,
                       const char* out_filename,
                       int32_t offset,
                       int32_t count,
                       const int32_t* variables);

    int read_rpg_switch(const char* filename,
                        int32_t offset,
                        int32_t count,
                        int8_t* ret);

    int write_rpg_switch(const char* in_filename,
                         const char* out_filename,
                         int32_t offset,
                         int32_t count,
                         const int8_t* switches);
}


/*  ------------------------------------------------------------   */
/*  Helper to create a simple .lsd file that contains the following
 *  data (all other fields are left at their default values):
 *
 *        system.variables = { 10, 20, 30, 40, 50 }
 *        system.switches  = { true, false, true }
 *
 *  The file is written to *path* and the function returns true
 *  on success.                                                   */
bool create_test_lsd(const std::filesystem::path& path)
{

    lcf::rpg::Save save;
    /*  variables – 5 32‑bit ints  */
    save.system.variables = {10, 20, 30, 40, 50};
    /*  switches – 3 bools  */
    save.system.switches   = {true, false, true};

    /*  Write the file   */
    return lcf::LSD_Reader::Save(path.string(),
                                   save,
                                   lcf::EngineVersion::e2k3,
                                   "");
}

/*  ------------------------------------------------------------   */
/*  Helper that prints a failure message and aborts the test   */
[[noreturn]] void fail(const char* msg)
{
    std::cerr << "FAIL: " << msg << '\n';
    std::_Exit(1);
}

/*  ------------------------------------------------------------   */
/*  Test the read_rpg_var function.  It should return the 5
 *  values we stored in the temporary file.              */
void test_read_rpg_var(const std::filesystem::path& lsd_path)
{
    const int32_t expected[5] = {10,20,30,40,50};
    int32_t got[5];

    int rc = read_rpg_var(lsd_path.string().c_str(),
                          0,            // offset
                          5,            // count
                          got);
    if (rc != 0) fail("read_rpg_var returned non‑zero");

    for (int i=0;i<5;++i)
        if (got[i] != expected[i])
            fail("read_rpg_var returned wrong value");
}

/*  ------------------------------------------------------------   */
/*  Test the write_rpg_var function.  We change the values to
 *  51,52,53,54,55, write the file to *new.lsd* and read it back. */
void test_write_rpg_var(const std::filesystem::path& src,
                       const std::filesystem::path& dst)
{
    const int32_t new_vals[5] = {51,52,53,54,55};
    int rc = write_rpg_var(src.string().c_str(),
                           dst.string().c_str(),
                           0, 5, new_vals);
    if (rc != 0) fail("write_rpg_var returned non‑zero");

    /*  Verify by reading the new file   */
    int32_t got[5];
    rc = read_rpg_var(dst.string().c_str(), 0, 5, got);
    if (rc != 0) fail("read after write_rpg_var failed");

    for (int i=0;i<5;++i)
        if (got[i] != new_vals[i])
            fail("write_rpg_var produced wrong data");
}

/*  ------------------------------------------------------------   */
/*  Test the read_rpg_switch function.  It should return
 *  {1,0,1}. */
void test_read_rpg_switch(const std::filesystem::path& lsd_path)
{
    int8_t expected[3] = {1,0,1};
    int8_t got[3];

    int rc = read_rpg_switch(lsd_path.string().c_str(),
                             0, 3, got);
    if (rc != 0) fail("read_rpg_switch returned non‑zero");

    for (int i=0;i<3;++i)
        if (got[i] != expected[i])
            fail("read_rpg_switch returned wrong value");
}

/*  ------------------------------------------------------------   */
/*  Test the write_rpg_switch function.  We flip the values to
 *  {0,1,0}, write to *new_switch.lsd* and read back.   */
void test_write_rpg_switch(const std::filesystem::path& src,
                           const std::filesystem::path& dst)
{
    int8_t new_vals[3] = {0,1,0};
    int rc = write_rpg_switch(src.string().c_str(),
                              dst.string().c_str(),
                              0, 3, new_vals);
    if (rc != 0) fail("write_rpg_switch returned non‑zero");

    int8_t got[3];
    rc = read_rpg_switch(dst.string().c_str(), 0, 3, got);
    if (rc != 0) fail("read after write_rpg_switch failed");

    for (int i=0;i<3;++i)
        if (got[i] != new_vals[i])
            fail("write_rpg_switch produced wrong data");
}

/*  ------------------------------------------------------------   */
/*  Main – create a temp file, run all tests, delete files.  */
int main()
{
    const std::filesystem::path tmp = "temp_test.lsd";
    const std::filesystem::path out = "temp_out.lsd";
    const std::filesystem::path out_sw = "temp_switch_out.lsd";

    /*  1.  Create the original lsd file   */
    if (!create_test_lsd(tmp))
        return 1;

    /*  2.  Run tests   */
    test_read_rpg_var(tmp);
    test_write_rpg_var(tmp, out);

    test_read_rpg_switch(tmp);
    test_write_rpg_switch(tmp, out_sw);

    /*  3.  Clean up   */
    std::filesystem::remove(tmp);
    std::filesystem::remove(out);
    std::filesystem::remove(out_sw);

    std::cout << "All tests passed.\n";
    return 0;
}

