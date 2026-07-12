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

#include <cstdint> // int8_t, int32_t, etc.
#include <cstring> // memcpy
#include <fstream>
#include <iostream>
#include <memory>
#include <string_view>

// ---------------  liblcf headers --------------------------------
#include "lcf/lsd/reader.h"
#include "lcf/reader_lcf.h"
#include "lcf/reader_util.h"
#include "lcf/rpg/save.h"
#include "lcf/rpg/savesystem.h"
#include "lcf/saveopt.h" // defines lcf::EngineVersion
#include "lcf/writer_lcf.h"

// ---------------  C interface -----------------------------------
extern "C" {

/*  -----------------------------------------------------------------
 *  read_rpg_var_lgs
 *
 *  Parameters
 *    filename : path to the .lgs file
 *    offset   : first variable index to read (0‑based)
 *    count    : how many variables to read
 *    ret      : caller‑supplied buffer; must be at least count*4 bytes
 *
 *  Returns  0 on success, -1 on error.
 ----------------------------------------------------------------- */
int read_rpg_var_lgs(const char *filename, int32_t offset, int32_t count,
                     int32_t *ret) {
  if (!filename || !ret || count <= 0 || offset < 0)
    return -1;

  std::ifstream file(filename, std::ios::binary);

  if (!file.is_open()) {
    std::cerr << "Unable to open File" << std::endl;
    return -1;
  }

  lcf::LcfReader reader(file);
  std::string header;
  reader.ReadString(header, reader.ReadInt());
  if (header.length() != 13 || header != "LcfGlobalSave") {
    std::cerr << "Not a valid Global Save file" << std::endl;
    return -1;
  }

  lcf::LcfReader::Chunk chunk;

  while (!reader.Eof()) {
    chunk.ID = reader.ReadInt();
    chunk.length = reader.ReadInt();
    switch (chunk.ID) {
    case 1: {
      reader.Skip(chunk, "RpgHelperScriptSkipSwitches");
      break;
    }
    case 2: {
      std::vector<int32_t> variables;
      reader.Read(variables, chunk.length);

      if (static_cast<size_t>(offset + count) > variables.size())
        return -1;

      // Copy the variables
      std::memcpy(ret, variables.data() + offset,
                  static_cast<size_t>(count) * sizeof(int32_t));
      return 0;
    }
    default:
      reader.Skip(chunk, "RpgHelperScriptSkipOtherChunks");
    }
  }

  std::cerr << "No Variable Section" << std::endl;
  return -1;
}

/*  -----------------------------------------------------------------
 *  read_rpg_switch_lgs
 *
 *  Parameters
 *    filename : path to the .lgs file
 *    offset   : first switch index to read (0‑based)
 *    count    : how many switches to read
 *    ret      : caller‑supplied buffer; must be at least count bytes
 *
 *  Returns  0 on success, -1 on error.
 ----------------------------------------------------------------- */
int read_rpg_switch_lgs(const char *filename, int32_t offset, int32_t count,
                        int8_t *ret) {
  if (!filename || !ret || count <= 0 || offset < 0)
    return -1;

  std::ifstream file(filename, std::ios::binary);
  if (!file.is_open()) {
    std::cerr << "Unable to open File" << std::endl;
    return -1;
  }

  /* read header ---------------------------------------------------- */
  lcf::LcfReader reader(file);
  std::string header;
  reader.ReadString(header, reader.ReadInt());
  if (header.length() != 13 || header != "LcfGlobalSave") {
    std::cerr << "Not a valid Global Save file" << std::endl;
    return -1;
  }

  /* read chunks ---------------------------------------------------- */
  lcf::LcfReader::Chunk chunk;
  std::vector<bool> switches;
  bool found_switches = false;

  while (!reader.Eof()) {
    chunk.ID = reader.ReadInt();
    chunk.length = reader.ReadInt();

    switch (chunk.ID) {
    case 1: { /* switches */
      reader.Read(switches, chunk.length);
      found_switches = true;
      break;
    }
    case 2: { /* variables – ignore for this call */
      reader.Skip(chunk, "RpgHelperScriptSkipVariables");
      break;
    }
    default: {
      reader.Skip(chunk, "RpgHelperScriptSkipOtherChunks");
      break;
    }
    }
  }

  if (!found_switches) {
    std::cerr << "No Switch Section" << std::endl;
    return -1;
  }

  if (static_cast<size_t>(offset + count) > switches.size())
    return -1;

  for (int32_t i = 0; i < count; ++i)
    ret[i] = switches[static_cast<size_t>(offset) + i] ? 1 : 0;

  return 0;
}

/*  -----------------------------------------------------------------
 *  write_rpg_var_lgs
 *
 *  Parameters
 *    in_filename   : original .lgs file
 *    out_filename  : destination .lgs file (may overwrite)
 *    offset        : first variable index to write
 *    count         : how many variables to write
 *    variables     : caller‑supplied array of new values
 *
 *  Returns  0 on success, -1 on error.
 ----------------------------------------------------------------- */
int write_rpg_var_lgs(const char *in_filename, const char *out_filename,
                      int32_t offset, int32_t count, const int32_t *variables) {
  if (!in_filename || !out_filename || !variables || count <= 0 || offset < 0)
    return -1;

  /* ---------- read the original file ---------- */
  std::ifstream in(in_filename, std::ios::binary);
  if (!in.is_open()) {
    std::cerr << "Unable to open File" << std::endl;
    return -1;
  }

  lcf::LcfReader reader(in);
  std::string header;
  reader.ReadString(header, reader.ReadInt());
  if (header.length() != 13 || header != "LcfGlobalSave") {
    std::cerr << "Not a valid Global Save file" << std::endl;
    return -1;
  }

  lcf::LcfReader::Chunk chunk;
  std::vector<bool> switches;
  std::vector<int32_t> vars;
  bool found_vars = false;

  while (!reader.Eof()) {
    chunk.ID = reader.ReadInt();
    chunk.length = reader.ReadInt();

    switch (chunk.ID) {
    case 1: { /* switches */
      reader.Read(switches, chunk.length);
      break;
    }
    case 2: { /* variables */
      reader.Read(vars, chunk.length);
      found_vars = true;
      break;
    }
    default: {
      reader.Skip(chunk, "CommandManiacControlGlobalSave");
      break;
    }
    }
  }

  if (!found_vars) {
    std::cerr << "No Variable Section" << std::endl;
    return -1;
  }

  if (static_cast<size_t>(offset + count) > vars.size())
    return -1;

  /* ---------- modify ---------- */
  std::memcpy(vars.data() + offset, variables,
              static_cast<size_t>(count) * sizeof(int32_t));

  /* ---------- close original ---------- */
  in.close();

  /* ---------- write the new file ---------- */
  std::ofstream out(out_filename, std::ios::binary);
  if (!out.is_open()) {
    std::cerr << "Unable to write file" << std::endl;
    return -1;
  }

  lcf::LcfWriter writer(out, lcf::EngineVersion::e2k3);
  writer.WriteInt(13);
  writer.Write(header);
  writer.WriteInt(1);
  writer.WriteInt(static_cast<int32_t>(switches.size()));
  writer.Write(switches);
  writer.WriteInt(2);
  writer.WriteInt(static_cast<int32_t>(vars.size() * sizeof(int32_t)));
  writer.Write(vars);

  return 0;
}

/*  -----------------------------------------------------------------
 *  write_rpg_switch_lgs
 *
 *  Parameters
 *    in_filename   : original .lgs file
 *    out_filename  : destination .lgs file
 *    offset        : first switch index to write
 *    count         : how many switches to write
 *    switches      : caller‑supplied array of new values (0/1)
 *
 *  Returns  0 on success, -1 on error.
 ----------------------------------------------------------------- */
int write_rpg_switch_lgs(const char *in_filename, const char *out_filename,
                         int32_t offset, int32_t count,
                         const int8_t *switches) {
  if (!in_filename || !out_filename || !switches || count <= 0 || offset < 0)
    return -1;

  /* ---------- read the original file ---------- */
  std::ifstream in(in_filename, std::ios::binary);
  if (!in.is_open()) {
    std::cerr << "Unable to open File" << std::endl;
    return -1;
  }

  lcf::LcfReader reader(in);
  std::string header;
  reader.ReadString(header, reader.ReadInt());
  if (header.length() != 13 || header != "LcfGlobalSave") {
    std::cerr << "Not a valid Global Save file" << std::endl;
    return -1;
  }

  lcf::LcfReader::Chunk chunk;
  std::vector<bool> switches_vec;
  std::vector<int32_t> vars;
  bool found_switches = false;

  while (!reader.Eof()) {
    chunk.ID = reader.ReadInt();
    chunk.length = reader.ReadInt();

    switch (chunk.ID) {
    case 1: { /* switches */
      reader.Read(switches_vec, chunk.length);
      found_switches = true;
      break;
    }
    case 2: { /* variables */
      reader.Read(vars, chunk.length);
      break;
    }
    default: {
      reader.Skip(chunk, "CommandManiacControlGlobalSave");
      break;
    }
    }
  }

  if (!found_switches) {
    std::cerr << "No Switch Section" << std::endl;
    return -1;
  }

  if (static_cast<size_t>(offset + count) > switches_vec.size())
    return -1;

  /* ---------- modify ---------- */
  for (int32_t i = 0; i < count; ++i)
    switches_vec[static_cast<size_t>(offset) + i] = (switches[i] != 0);

  /* ---------- close original ---------- */
  in.close();

  /* ---------- write the new file ---------- */
  std::ofstream out(out_filename, std::ios::binary);
  if (!out.is_open()) {
    std::cerr << "Unable to write file" << std::endl;
    return -1;
  }

  lcf::LcfWriter writer(out, lcf::EngineVersion::e2k3);
  writer.WriteInt(13);
  writer.Write(header);
  writer.WriteInt(1);
  writer.WriteInt(static_cast<int32_t>(switches_vec.size()));
  writer.Write(switches_vec);
  writer.WriteInt(2);
  writer.WriteInt(static_cast<int32_t>(vars.size() * sizeof(int32_t)));
  writer.Write(vars);

  return 0;
}

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
int read_rpg_var(const char *filename, int32_t offset, int32_t count,
                 int32_t *ret) {
  if (!filename || !ret || count <= 0 || offset < 0)
    return -1;

  // Load the save file
  std::unique_ptr<lcf::rpg::Save> savePtr =
      lcf::LSD_Reader::Load(std::string_view(filename));
  if (!savePtr)
    return -1; // file not found / parse error

  const std::vector<int32_t> &vars = savePtr->system.variables;
  if (static_cast<size_t>(offset + count) > vars.size())
    return -1;

  // Copy the variables
  std::memcpy(ret, vars.data() + offset,
              static_cast<size_t>(count) * sizeof(int32_t));
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
int write_rpg_var(const char *in_filename, const char *out_filename,
                  int32_t offset, int32_t count, const int32_t *variables) {
  if (!in_filename || !out_filename || !variables || count <= 0 || offset < 0)
    return -1;

  // Load the original file
  std::unique_ptr<lcf::rpg::Save> savePtr =
      lcf::LSD_Reader::Load(std::string_view(in_filename));
  if (!savePtr)
    return -1;

  std::vector<int32_t> &vars = savePtr->system.variables;
  if (static_cast<size_t>(offset + count) > vars.size())
    return -1;

  // Copy the new values
  std::memcpy(vars.data() + offset, variables,
              static_cast<size_t>(count) * sizeof(int32_t));

  // Write the modified file
  if (!lcf::LSD_Reader::Save(std::string_view(out_filename), *savePtr,
                             lcf::EngineVersion::e2k3, // choose a valid engine
                             ""))
    return -1; // default encoding

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
int read_rpg_switch(const char *filename, int32_t offset, int32_t count,
                    int8_t *ret) {
  if (!filename || !ret || count <= 0 || offset < 0)
    return -1;

  std::unique_ptr<lcf::rpg::Save> savePtr =
      lcf::LSD_Reader::Load(std::string_view(filename));
  if (!savePtr)
    return -1;

  const std::vector<bool> &sw = savePtr->system.switches;
  if (static_cast<size_t>(offset + count) > sw.size())
    return -1;

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
int write_rpg_switch(const char *in_filename, const char *out_filename,
                     int32_t offset, int32_t count, const int8_t *switches) {
  if (!in_filename || !out_filename || !switches || count <= 0 || offset < 0)
    return -1;

  std::unique_ptr<lcf::rpg::Save> savePtr =
      lcf::LSD_Reader::Load(std::string_view(in_filename));
  if (!savePtr)
    return -1;

  std::vector<bool> &sw = savePtr->system.switches;
  if (static_cast<size_t>(offset + count) > sw.size())
    return -1;

  for (int32_t i = 0; i < count; ++i)
    sw[static_cast<size_t>(offset) + i] = (switches[i] != 0);

  if (!lcf::LSD_Reader::Save(std::string_view(out_filename), *savePtr,
                             lcf::EngineVersion::e2k3, ""))
    return -1;

  return 0;
}
}

/*********************************************************************
 *  End of rpg_lsd_io.cpp
 *********************************************************************/
