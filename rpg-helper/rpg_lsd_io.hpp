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

/* ----------------------------------------------------------------------------- */
/*  Helper – split a string into tokens                                */
/* ----------------------------------------------------------------------------- */
static std::vector<std::string> split(const std::string& s)
{
    std::istringstream iss(s);
    std::string token;
    std::vector<std::string> tokens;
    while (iss >> token)
        tokens.push_back(token);
    return tokens;
}

/* ----------------------------------------------------------------------------- */
/*  Parse an array that is written like "[ 1 2 3 4 ]"  */
/* ----------------------------------------------------------------------------- */
template<typename T>
static bool parse_array(const std::string& s, std::vector<T>& out)
{
    std::string str = s;
    // Trim leading/trailing whitespace
    auto start = str.find_first_not_of(" \t\r\n");
    auto end   = str.find_last_not_of(" \t\r\n");
    if (start == std::string::npos || end == std::string::npos)
        return false;
    str = str.substr(start, end - start + 1);

    // Must start with '[' and end with ']'
    if (str.front() != '[' || str.back() != ']')
        return false;
    str = str.substr(1, str.size() - 2);   // strip brackets

    std::vector<std::string> toks = split(str);
    out.clear();
    out.reserve(toks.size());
    for (const auto& tok : toks)
    {
        if constexpr (std::is_same_v<T, int32_t>)
        {
            char* endptr = nullptr;
            long val = std::strtol(tok.c_str(), &endptr, 10);
            if (*endptr != '\0')
                return false;
            out.push_back(static_cast<int32_t>(val));
        }
        else if constexpr (std::is_same_v<T, int8_t>)
        {
            if (tok == "1" || tok == "T" || tok == "t")
                out.push_back(1);
            else if (tok == "0" || tok == "F" || tok == "f")
                out.push_back(0);
            else
                return false;
        }
        else
            return false;
    }
    return true;
}

#endif
