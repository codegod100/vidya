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
option(VIDYA_BUILD_ANDROID_APP "Build libmain.so for an Android NativeActivity" OFF)
set(VIDYA_JOLT_BOOT_OBJECT "" CACHE FILEPATH "ARM64 ELF object containing jolt.boot")
set(VIDYA_CHEZ_ANDROID "" CACHE PATH "Chez tarm64le build root")
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

  set(RLIMGUI_SOURCE "${rlimgui_source_SOURCE_DIR}/rlImGui.cpp")
  if(ANDROID)
    # rlImGui treats every non-Emscripten platform as desktop and asks raylib
    # for a window DPI scale. Android does not implement that API, returning a
    # zero scale that clips every ImGui draw command. Keep the dependency
    # immutable and patch a generated build copy to use Android's 1x framebuffer
    # scale, matching its existing Emscripten policy.
    file(READ "${RLIMGUI_SOURCE}" RLIMGUI_ANDROID_SOURCE)
    string(REPLACE
      "#if defined(__EMSCRIPTEN__)"
      "#if defined(__EMSCRIPTEN__) || defined(PLATFORM_ANDROID)"
      RLIMGUI_ANDROID_SOURCE "${RLIMGUI_ANDROID_SOURCE}")
    set(RLIMGUI_SOURCE "${CMAKE_CURRENT_BINARY_DIR}/rlImGui_android.cpp")
    file(WRITE "${RLIMGUI_SOURCE}" "${RLIMGUI_ANDROID_SOURCE}")
  endif()
  add_library(rlimgui STATIC "${RLIMGUI_SOURCE}")
  target_compile_definitions(rlimgui PRIVATE NO_FONT_AWESOME)
  # Android NativeActivity windows report fullscreen, but raylib's Android
  # backend does not implement monitor enumeration or GetWindowScaleDPI.
  # rlImGui's DRM path uses the actual screen dimensions and a 1x scale, which
  # is the correct policy for this backend too.
  if(ANDROID)
    target_compile_definitions(rlimgui PRIVATE PLATFORM_DRM)
  endif()
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

if(ANDROID)
  add_library(vidya STATIC ${VIDYA_SOURCE})
else()
  add_library(vidya SHARED ${VIDYA_SOURCE})
endif()
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

if(ANDROID AND VIDYA_BUILD_ANDROID_APP)
  if(NOT VIDYA_JOLT_BOOT_OBJECT OR NOT EXISTS "${VIDYA_JOLT_BOOT_OBJECT}")
    message(FATAL_ERROR "VIDYA_JOLT_BOOT_OBJECT must name the embedded ARM64 Jolt boot object")
  endif()
  if(NOT VIDYA_CHEZ_ANDROID OR
      NOT EXISTS "${VIDYA_CHEZ_ANDROID}/tarm64le/boot/tarm64le/libkernel.a")
    message(FATAL_ERROR "VIDYA_CHEZ_ANDROID must name a Chez tarm64le cross build")
  endif()

  add_library(main SHARED
    android/jolt_main.c
    "${VIDYA_JOLT_BOOT_OBJECT}")
  set_source_files_properties("${VIDYA_JOLT_BOOT_OBJECT}"
    PROPERTIES EXTERNAL_OBJECT TRUE GENERATED TRUE)
  target_include_directories(main PRIVATE
    "${VIDYA_CHEZ_ANDROID}/boot/tarm64le")
  target_link_libraries(main PRIVATE
    vidya
    "${VIDYA_CHEZ_ANDROID}/tarm64le/boot/tarm64le/libkernel.a"
    "${VIDYA_CHEZ_ANDROID}/lz4/lib/liblz4.a"
    android log EGL GLESv2 OpenSLES z dl m)
  target_link_options(main PRIVATE
    "-Wl,-u,ANativeActivity_onCreate"
    "-Wl,--export-dynamic"
    "-Wl,--no-undefined")
  set_target_properties(main PROPERTIES OUTPUT_NAME main)
endif()

include(GNUInstallDirs)
install(TARGETS vidya EXPORT vidya-targets)
install(FILES include/vidya.h DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
