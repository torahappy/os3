//  test_rpg_lsd_io.cpp
//
//  Compile with something like:
//
//      g++ -std=c++20 -I<path-to-lcf-headers> -L<path-to-lcf-lib>
//          -lrpg_lsd_io -lrpg_lcf -o test_rpg_lsd_io test_rpg_lsd_io.cpp
//
//  (The `lcf` library is required – the code below only uses its
//   public writer API to generate the test files.)

#define CATCH_CONFIG_ENABLE_EXCEPTIONS
#include "catch_amalgamated.hpp"
#include "rpg_lsd_io.hpp"

#include <cstdint>          // int8_t, int32_t
#include <cstring>
#include <fstream>
#include <vector>
#include <filesystem>
#include <string_view>
#include <iostream>

// -------------  library includes --------------------------------
#include "lcf/lsd/reader.h"
#include "lcf/reader_lcf.h"
#include "lcf/reader_util.h"
#include "lcf/rpg/save.h"
#include "lcf/rpg/savesystem.h"
#include "lcf/saveopt.h"
#include "lcf/writer_lcf.h"

// -------------  exported C interface --------------------------------

// -------------  helpers for generating the test files --------------------------------
namespace {

    //  Create a minimal .lgs file containing the test data
    void create_test_lgs(const std::string &path)
    {
        std::ofstream out(path, std::ios::binary);
        REQUIRE(out.is_open());   // helper will abort test if we fail

        lcf::LcfWriter writer(out, lcf::EngineVersion::e2k3);
        writer.WriteInt(13);                     // magic header size
        writer.Write("LcfGlobalSave");           // magic string

        // --- Switch chunk ---------------------------------
        std::vector<bool> sw = {true, false, true, true};
        writer.WriteInt(1);                       // chunk ID
        writer.WriteInt(static_cast<int32_t>(sw.size()));
        writer.Write(sw);

        // --- Variable chunk --------------------------------
        std::vector<int32_t> vars = {10, 20, 30, 40};
        writer.WriteInt(2);                       // chunk ID
        writer.WriteInt(static_cast<int32_t>(vars.size() * sizeof(int32_t)));
        writer.Write(vars);
    }

    //  Create a minimal .lsd file containing the same test data
    void create_test_lsd(const std::string &path)
    {
        lcf::rpg::Save save;
        //  variables
        save.system.variables = {10, 20, 30, 40};
        //  switches
        save.system.switches   = {true, false, true, true};

        bool ok = lcf::LSD_Reader::Save(
            std::string_view(path), save,
            lcf::EngineVersion::e2k3, /* encoding */ "");
        REQUIRE(ok);
    }

}   // anonymous namespace

// -------------  test cases --------------------------------
TEST_CASE("lgs functions – read variables", "[lgs][var]")
{
    const char *fname = "test1.lgs";
    int32_t vals[4];

    int rc = read_rpg_var_lgs(fname, 0, 4, vals);
    REQUIRE(rc == 0);
    REQUIRE(vals[0] == 10);
    REQUIRE(vals[1] == 20);
    REQUIRE(vals[2] == 30);
    REQUIRE(vals[3] == 40);
}

TEST_CASE("lgs functions – read switches", "[lgs][switch]")
{
    const char *fname = "test1.lgs";
    int8_t sw[4];

    int rc = read_rpg_switch_lgs(fname, 0, 4, sw);
    REQUIRE(rc == 0);
    REQUIRE(sw[0] == 1);   // true
    REQUIRE(sw[1] == 0);   // false
    REQUIRE(sw[2] == 1);   // true
    REQUIRE(sw[3] == 1);   // true
}

TEST_CASE("lgs functions – write variables and re‑read", "[lgs][write]")
{
    const char *in  = "test1.lgs";
    const char *out = "test1.lgs.tmp";
    int32_t newVals[4] = {100, 200, 300, 400};

    int rc = write_rpg_var_lgs(in, out, 0, 4, newVals);
    REQUIRE(rc == 0);

    // read back
    int32_t vals[4];
    rc = read_rpg_var_lgs(out, 0, 4, vals);
    REQUIRE(rc == 0);
    REQUIRE(vals[0] == 100);
    REQUIRE(vals[1] == 200);
    REQUIRE(vals[2] == 300);
    REQUIRE(vals[3] == 400);

    // cleanup
    std::filesystem::remove(out);
}

TEST_CASE("lgs functions – write switches and re‑read", "[lgs][write]")
{
    const char *in  = "test1.lgs";
    const char *out = "test1.lgs.tmp";
    int8_t newSw[4] = {0, 1, 0, 1};   // false, true, false, true

    int rc = write_rpg_switch_lgs(in, out, 0, 4, newSw);
    REQUIRE(rc == 0);

    int8_t sw[4];
    rc = read_rpg_switch_lgs(out, 0, 4, sw);
    REQUIRE(rc == 0);
    REQUIRE(sw[0] == 0);
    REQUIRE(sw[1] == 1);
    REQUIRE(sw[2] == 0);
    REQUIRE(sw[3] == 1);

    std::filesystem::remove(out);
}

TEST_CASE("lgs functions – partial read (offset=1)", "[lgs][partial]")
{
    const char *fname = "test1.lgs";
    int32_t vals[2];

    int rc = read_rpg_var_lgs(fname, 1, 2, vals);
    REQUIRE(rc == 0);
    REQUIRE(vals[0] == 20);   // var[2]
    REQUIRE(vals[1] == 30);   // var[3]
}

TEST_CASE("lgs functions – out‑of‑range read returns error", "[lgs][error]")
{
    const char *fname = "test1.lgs";
    int32_t vals[1];
    int rc = read_rpg_var_lgs(fname, 4000, 1, vals);   // offset 4000 is past the last var
    REQUIRE(rc == -1);
}

TEST_CASE("lsd functions – read variables", "[lsd][var]")
{
    const char *fname = "test1.lsd";
    int32_t vals[4];
    int rc = read_rpg_var(fname, 0, 4, vals);
    REQUIRE(rc == 0);
    REQUIRE(vals[0] == 10);
    REQUIRE(vals[1] == 20);
    REQUIRE(vals[2] == 30);
    REQUIRE(vals[3] == 40);
}

TEST_CASE("lsd functions – read switches", "[lsd][switch]")
{
    const char *fname = "test1.lsd";
    int8_t sw[4];
    int rc = read_rpg_switch(fname, 0, 4, sw);
    REQUIRE(rc == 0);
    REQUIRE(sw[0] == 1);
    REQUIRE(sw[1] == 0);
    REQUIRE(sw[2] == 1);
    REQUIRE(sw[3] == 1);
}

TEST_CASE("lsd functions – write variables and re‑read", "[lsd][write]")
{
    const char *in  = "test1.lsd";
    const char *out = "test1.lsd.tmp";
    int32_t newVals[4] = {1000, 2000, 3000, 4000};

    int rc = write_rpg_var(in, out, 0, 4, newVals);
    REQUIRE(rc == 0);

    int32_t vals[4];
    rc = read_rpg_var(out, 0, 4, vals);
    REQUIRE(rc == 0);
    REQUIRE(vals[0] == 1000);
    REQUIRE(vals[1] == 2000);
    REQUIRE(vals[2] == 3000);
    REQUIRE(vals[3] == 4000);

    std::filesystem::remove(out);
}

TEST_CASE("lsd functions – write switches and re‑read", "[lsd][write]")
{
    const char *in  = "test1.lsd";
    const char *out = "test1.lsd.tmp";
    int8_t newSw[4] = {1, 0, 1, 0};

    int rc = write_rpg_switch(in, out, 0, 4, newSw);
    REQUIRE(rc == 0);

    int8_t sw[4];
    rc = read_rpg_switch(out, 0, 4, sw);
    REQUIRE(rc == 0);
    REQUIRE(sw[0] == 1);
    REQUIRE(sw[1] == 0);
    REQUIRE(sw[2] == 1);
    REQUIRE(sw[3] == 0);

    std::filesystem::remove(out);
}

TEST_CASE("lsd functions – partial read (offset=1)", "[lsd][partial]")
{
    const char *fname = "test1.lsd";
    int32_t vals[2];
    int rc = read_rpg_var(fname, 1, 2, vals);
    REQUIRE(rc == 0);
    REQUIRE(vals[0] == 20);
    REQUIRE(vals[1] == 30);
}

TEST_CASE("lsd functions – out‑of‑range read returns error", "[lsd][error]")
{
    const char *fname = "test1.lsd";
    int32_t vals[1];
    int rc = read_rpg_var(fname, 4, 1, vals);   // offset 4 is past the last var
    REQUIRE(rc == -1);
}

// -------------  main entry point (Catch2 generates it automatically) --------------------------------


