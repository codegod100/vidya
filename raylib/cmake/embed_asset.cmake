# Turn a binary asset into a C source so it ships inside the library. Android
# has no data directory to read from, and a bundled font must not depend on the
# working directory the application happened to start in.
#
#   cmake -DASSET=<in> -DOUTPUT=<out.c> -DSYMBOL=<name> -P embed_asset.cmake

file(READ "${ASSET}" hex HEX)
string(LENGTH "${hex}" hex_length)
math(EXPR length "${hex_length} / 2")
string(REGEX REPLACE "(..)" "0x\\1," bytes "${hex}")
# MSVC caps a logical source line at 64k characters, so wrap at 16 bytes.
string(REGEX REPLACE "((0x..,){16})" "\\1\n" bytes "${bytes}")
get_filename_component(asset_name "${ASSET}" NAME)
file(WRITE "${OUTPUT}"
  "/* Generated from ${asset_name}. Do not edit. */\n"
  "const unsigned char ${SYMBOL}[] = {\n${bytes}\n};\n"
  "const unsigned int ${SYMBOL}_size = ${length}u;\n")
