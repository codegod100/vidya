cmake_minimum_required(VERSION 3.20)
project(vidya_raylib VERSION 0.1.0 LANGUAGES C)

# Zig 0.16's bundled linker currently crashes on CMake's GNU-style
# `--dependency-file` link argument. Object dependency files still work; only
# disable the optional linker depfile when Zig is acting as the C compiler.
get_filename_component(VIDYA_C_COMPILER_NAME "${CMAKE_C_COMPILER}" NAME)
if(VIDYA_C_COMPILER_NAME STREQUAL "zig")
  set(CMAKE_C_LINKER_DEPFILE_SUPPORTED FALSE)
  set(CMAKE_C_LINKER_DEPFILE_FLAGS "")
endif()

option(VIDYA_FETCH_RAYLIB "Fetch raylib when it is not installed" OFF)
option(VIDYA_BUILD_EXAMPLE "Build the Vidya showcase" ON)

find_package(raylib 5.0 QUIET)
if(NOT raylib_FOUND AND VIDYA_FETCH_RAYLIB)
  include(FetchContent)
  FetchContent_Declare(
    raylib
    GIT_REPOSITORY https://github.com/raysan5/raylib.git
    GIT_TAG 5.5
    GIT_SHALLOW TRUE
  )
  set(BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
  FetchContent_MakeAvailable(raylib)
endif()

if(NOT TARGET raylib)
  message(FATAL_ERROR
    "raylib was not found. Install raylib or configure with -DVIDYA_FETCH_RAYLIB=ON")
endif()

# raylib's nested project reloads compiler metadata, so apply this after it has
# been configured as well as above.
if(VIDYA_C_COMPILER_NAME STREQUAL "zig")
  set(CMAKE_C_LINKER_DEPFILE_SUPPORTED FALSE)
  set(CMAKE_C_LINKER_DEPFILE_FLAGS "")
endif()

add_library(vidya SHARED src/vidya.c)
add_library(vidya::vidya ALIAS vidya)
target_include_directories(vidya PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:include>)
target_link_libraries(vidya PRIVATE raylib)
target_compile_features(vidya PUBLIC c_std_99)
set_target_properties(vidya PROPERTIES
  C_VISIBILITY_PRESET hidden
  VISIBILITY_INLINES_HIDDEN YES)

if(VIDYA_BUILD_EXAMPLE)
  add_executable(vidya-showcase examples/showcase.c)
  target_link_libraries(vidya-showcase PRIVATE vidya)
endif()

include(GNUInstallDirs)
install(TARGETS vidya EXPORT vidya-targets)
install(FILES include/vidya.h DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
