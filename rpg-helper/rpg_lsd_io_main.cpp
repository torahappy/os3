
#include <cstdint> // int8_t, int32_t, etc.
#include <cstdlib>
#include <cstring> // memcpy
#include <fstream>
#include <iostream>
#include <memory>
#include <sstream>
#include <string>
#include <string_view>
#include <vector>

#include "./rpg_lsd_io.hpp"


/* -----------------------------------------------------------------------------
 */
/*  main() – entry point */
/* -----------------------------------------------------------------------------
 */
int main(int argc, char **argv) {
  if (argc < 2) {
    std::cerr << "Usage: " << argv[0] << " <function> [args…]\n";
    return 1;
  }

  std::string func = argv[1];

  /* ------------------------------------------------------------ */
  /*  read_rpg_var_lgs   (filename, offset, count)                     */
  /* ------------------------------------------------------------ */
  if (func == "read_rpg_var_lgs" || func == "read_rpg_switch_lgs" ||
      func == "read_rpg_var" || func == "read_rpg_switch") {
    if (argc != 5) {
      std::cerr << "Expected 4 arguments for " << func << '\n';
      return 1;
    }

    const char *filename = argv[2];
    int32_t offset = std::stoi(argv[3]);
    int32_t count = std::stoi(argv[4]);

    /* Allocate buffer for the result */
    int rc;
    if (func == "read_rpg_var_lgs" || func == "read_rpg_var") {
      std::vector<int32_t> buf(count);
      if (func == "read_rpg_var_lgs") {
        rc = read_rpg_var_lgs(filename, offset, count, buf.data());
      } else {
        rc = read_rpg_var(filename, offset, count, buf.data());
      }
      if (rc != 0) {
        std::cerr << "Read failed\n";
        return 1;
      }
      for (int i = 0; i < count; ++i) {
        std::cout << buf[i];
        if (i + 1 < count)
          std::cout << ' ';
      }
      std::cout << '\n';
    } else {
      int8_t* bools = (int8_t *)malloc(count);
      if (func == "read_rpg_switch_lgs") {
        rc = read_rpg_switch_lgs(filename, offset, count, bools);
      } else {
        rc = read_rpg_switch(filename, offset, count, bools);
      }
      if (rc != 0) {
        std::cerr << "Read failed\n";
        return 1;
      }
      for (int i = 0; i < count; ++i) {
        std::cout << ((bool*)bools)[i];
        if (i + 1 < count)
          std::cout << ' ';
      }
      std::cout << '\n';
      free(bools);
    }
  }

  /* ------------------------------------------------------------ */
  /*  write_rpg_var_lgs   (in, out, offset, count, [values…])      */
  /*  write_rpg_switch_lgs (in, out, offset, count, [values…])    */
  /*  write_rpg_var      (in, out, offset, count, [values…])       */
  /*  write_rpg_switch   (in, out, offset, count, [values…])       */
  /* ------------------------------------------------------------ */
  else if (func == "write_rpg_var_lgs" || func == "write_rpg_switch_lgs" ||
           func == "write_rpg_var" || func == "write_rpg_switch") {
    if (argc < 6) {
      std::cerr << "Expected at least 5 arguments for " << func << '\n';
      return 1;
    }

    const char *in_file = argv[2];
    const char *out_file = argv[3];
    int32_t offset = std::stoi(argv[4]);
    int32_t count = std::stoi(argv[5]);

    /* The array argument may be supplied as a single token like
       "[ 1 2 3 4 ]" or as a comma‑separated list of tokens after the
       count.  We accept both forms. */

    std::vector<int32_t> int_buf;
    std::vector<int8_t> bool_buf;

    bool parsed = false;

    /* try to parse the next argument as a bracketed array */
    if (argc > 6) {
      std::string array_token = argv[6];
      if (func == "write_rpg_var_lgs" || func == "write_rpg_var") {
        parsed = parse_array<int32_t>(array_token, int_buf);
        if (parsed && (int)int_buf.size() == count) {
          /* ok */
        } else if (!parsed && argc >= 7) {
          /* maybe the array is split across multiple tokens */
          std::string combined;
          for (int i = 6; i < argc; ++i)
            combined += std::string(argv[i]) + ' ';
          parsed = parse_array<int32_t>(combined, int_buf);
          if (!(parsed && (int)int_buf.size() == count))
            parsed = false;
        }
      } else /* write_rpg_switch_* */
      {
        parsed = parse_array<int8_t>(array_token, bool_buf);
        if (parsed && (int)bool_buf.size() == count) {
          /* ok */
        } else if (!parsed && argc >= 7) {
          std::string combined;
          for (int i = 6; i < argc; ++i)
            combined += std::string(argv[i]) + ' ';
          parsed = parse_array<int8_t>(combined, bool_buf);
          if (!(parsed && (int)bool_buf.size() == count))
            parsed = false;
        }
      }
    }

    if (!parsed) {
      std::cerr << "Could not parse array of values for " << func << '\n';
      return 1;
    }

    /* Call the correct write‑function */
    int rc = -1;
    if (func == "write_rpg_var_lgs")
      rc = write_rpg_var_lgs(in_file, out_file, offset, count, int_buf.data());
    else if (func == "write_rpg_switch_lgs")
      rc = write_rpg_switch_lgs(in_file, out_file, offset, count,
                                bool_buf.data());
    else if (func == "write_rpg_var")
      rc = write_rpg_var(in_file, out_file, offset, count, int_buf.data());
    else /* write_rpg_switch */
      rc = write_rpg_switch(in_file, out_file, offset, count, bool_buf.data());

    if (rc != 0) {
      std::cerr << "Write failed\n";
      return 1;
    }
    return 0;
  }

  std::cerr << "Unknown function: " << func << '\n';
  return 1;
}

