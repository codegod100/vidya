## The small part of raylib's ABI used by Vidya.  Keeping this private avoids
## making users install a Nim raylib wrapper just to use the library.

const raylibLibrary =
  when defined(windows): "raylib.dll"
  elif defined(macosx): "libraylib.dylib"
  else: "libraylib.so"

type
  Color* {.bycopy.} = object
    r*, g*, b*, a*: uint8

  Vector2* {.bycopy.} = object
    x*, y*: cfloat

  Rectangle* {.bycopy.} = object
    x*, y*, width*, height*: cfloat

  Texture* {.bycopy.} = object
    id*: cuint
    width*, height*, mipmaps*, format*: cint

  Font* {.bycopy.} = object
    baseSize*, glyphCount*, glyphPadding*: cint
    texture*: Texture
    recs*: ptr Rectangle
    glyphs*: pointer

const
  FlagWindowResizable* = 0x00000004.cuint
  FlagMsaa4xHint* = 0x00000020.cuint
  FlagVsyncHint* = 0x00000040.cuint
  MouseButtonLeft* = 0.cint
  KeyNull* = 0.cint
  KeyBackspace* = 259.cint
  TextureFilterBilinear* = 1.cint

{.push cdecl, importc, dynlib: raylibLibrary.}
proc setConfigFlags*(flags: cuint) {.importc: "SetConfigFlags".}
proc initWindow*(width, height: cint; title: cstring) {.importc: "InitWindow".}
proc rlCloseWindow*() {.importc: "CloseWindow".}
proc isWindowReady*(): bool {.importc: "IsWindowReady".}
proc windowShouldClose*(): bool {.importc: "WindowShouldClose".}
proc rlSetTargetFPS*(fps: cint) {.importc: "SetTargetFPS".}
proc setExitKey*(key: cint) {.importc: "SetExitKey".}
proc getScreenWidth*(): cint {.importc: "GetScreenWidth".}
proc getScreenHeight*(): cint {.importc: "GetScreenHeight".}
proc beginDrawing*() {.importc: "BeginDrawing".}
proc endDrawing*() {.importc: "EndDrawing".}
proc clearBackground*(color: Color) {.importc: "ClearBackground".}

proc fileExists*(path: cstring): bool {.importc: "FileExists".}
proc loadFontEx*(path: cstring; fontSize: cint; codepoints: ptr cint;
                 codepointCount: cint): Font {.importc: "LoadFontEx".}
proc getFontDefault*(): Font {.importc: "GetFontDefault".}
proc isFontValid*(font: Font): bool {.importc: "IsFontValid".}
proc unloadFont*(font: Font) {.importc: "UnloadFont".}
proc setTextureFilter*(texture: Texture; filter: cint) {.importc: "SetTextureFilter".}
proc measureTextEx*(font: Font; text: cstring; fontSize, spacing: cfloat): Vector2 {.importc: "MeasureTextEx".}
proc drawTextEx*(font: Font; text: cstring; position: Vector2;
                 fontSize, spacing: cfloat; tint: Color) {.importc: "DrawTextEx".}

proc getMousePosition*(): Vector2 {.importc: "GetMousePosition".}
proc checkCollisionPointRec*(point: Vector2; rect: Rectangle): bool {.importc: "CheckCollisionPointRec".}
proc isMouseButtonDown*(button: cint): bool {.importc: "IsMouseButtonDown".}
proc isMouseButtonPressed*(button: cint): bool {.importc: "IsMouseButtonPressed".}
proc isMouseButtonReleased*(button: cint): bool {.importc: "IsMouseButtonReleased".}
proc isKeyPressed*(key: cint): bool {.importc: "IsKeyPressed".}
proc getCharPressed*(): cint {.importc: "GetCharPressed".}

proc drawRectangleRec*(rect: Rectangle; color: Color) {.importc: "DrawRectangleRec".}
proc drawRectangleRounded*(rect: Rectangle; roundness: cfloat; segments: cint;
                           color: Color) {.importc: "DrawRectangleRounded".}
proc drawRectangleRoundedLinesEx*(rect: Rectangle; roundness: cfloat;
                                  segments: cint; lineThick: cfloat;
                                  color: Color) {.importc: "DrawRectangleRoundedLinesEx".}
proc drawLineEx*(startPos, endPos: Vector2; thick: cfloat; color: Color) {.importc: "DrawLineEx".}
proc drawCircleV*(center: Vector2; radius: cfloat; color: Color) {.importc: "DrawCircleV".}
proc drawCircleLinesV*(center: Vector2; radius: cfloat; color: Color) {.importc: "DrawCircleLinesV".}
{.pop.}
