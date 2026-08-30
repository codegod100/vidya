cmake_minimum_required(VERSION 3.20)
project(vidya_raylib VERSION 0.1.0 LANGUAGES C CXX)

# Zig 0.16's bundled linker currently crashes on CMake's GNU-style
# `--dependency-file` link argument. Object dependency files still work; only
# disable the optional linker depfile when Zig is acting as the C compiler.
get_filename_component(VIDYA_C_COMPILER_NAME "${CMAKE_C_COMPILER}" NAME)
if(VIDYA_C_COMPILER_NAME STREQUAL "zig")
  set(CMAKE_C_LINKER_DEPFILE_SUPPORTED FALSE)
  set(CMAKE_C_LINKER_DEPFILE_FLAGS "")
  set(CMAKE_CXX_LINKER_DEPFILE_SUPPORTED FALSE)
  set(CMAKE_CXX_LINKER_DEPFILE_FLAGS "")
endif()

option(VIDYA_FETCH_RAYLIB "Fetch raylib when it is not installed" OFF)
option(VIDYA_BUILD_EXAMPLE "Build the Vidya showcase" ON)
set(VIDYA_BACKEND "cimgui" CACHE STRING "UI backend: cimgui or direct")
set_property(CACHE VIDYA_BACKEND PROPERTY STRINGS cimgui direct)

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
  set(CMAKE_CXX_LINKER_DEPFILE_SUPPORTED FALSE)
  set(CMAKE_CXX_LINKER_DEPFILE_FLAGS "")
endif()

if(VIDYA_BACKEND STREQUAL "cimgui")
  include(FetchContent)
  set(IMGUI_STATIC yes CACHE STRING "" FORCE)
  set(CIMGUI_TEST no CACHE STRING "" FORCE)
  FetchContent_Declare(
    cimgui
    GIT_REPOSITORY https://github.com/cimgui/cimgui.git
    GIT_TAG bb4d4848b759e037a6cc26d23be0387a148ac39f
    GIT_SUBMODULES_RECURSE TRUE
  )
  FetchContent_Declare(
    rlimgui_source
    GIT_REPOSITORY https://github.com/raylib-extras/rlImGui.git
    GIT_TAG 1550009359ad975927f7f0e4a3f47e3f27123ea9
    GIT_SHALLOW TRUE
  )
  FetchContent_MakeAvailable(cimgui rlimgui_source)

  add_library(rlimgui STATIC
    ${rlimgui_source_SOURCE_DIR}/rlImGui.cpp)
  target_compile_definitions(rlimgui PRIVATE NO_FONT_AWESOME)
  target_include_directories(rlimgui PUBLIC
    ${rlimgui_source_SOURCE_DIR}
    ${cimgui_SOURCE_DIR}/imgui)
  target_link_libraries(rlimgui PUBLIC raylib cimgui)
  set(VIDYA_SOURCE src/vidya_cimgui.c)
elseif(VIDYA_BACKEND STREQUAL "direct")
  set(VIDYA_SOURCE src/vidya.c)
else()
  message(FATAL_ERROR "VIDYA_BACKEND must be cimgui or direct")
endif()

add_library(vidya SHARED ${VIDYA_SOURCE})
add_library(vidya::vidya ALIAS vidya)
target_include_directories(vidya PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:include>)
target_link_libraries(vidya PRIVATE raylib)
if(VIDYA_BACKEND STREQUAL "cimgui")
  target_compile_definitions(vidya PRIVATE CIMGUI_DEFINE_ENUMS_AND_STRUCTS)
  target_include_directories(vidya PRIVATE
    ${cimgui_SOURCE_DIR}
    ${rlimgui_source_SOURCE_DIR})
  target_link_libraries(vidya PRIVATE cimgui rlimgui)
endif()
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
