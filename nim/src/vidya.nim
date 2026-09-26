## Vidya's native Nim implementation.
##
## This module owns the window, layout, theme, drawing, and interaction state.
## It also exports the stable ``vidya_*`` C ABI, so programs in other
## languages can use a library built from this file.

import std/[math, unicode]
import vidya/raylib

type
  Mode* {.pure.} = enum dark = 0, light = 1
  ButtonKind* {.pure.} = enum normal = 0, primary = 1, destructive = 2
  RenderProc* = proc() {.closure.}

  Palette = object
    windowBg, viewBg, cardBg: Color
    accent, accentFg, accentHover, accentActive: Color
    destructive, success: Color
    text, textSecondary, textDisabled: Color
    borderSoft, buttonBg, buttonHover, buttonActive, buttonFg: Color

  Layout = object
    x, y, width: cfloat

  CardState = object
    parent: Layout
    top: cfloat
    parentFill: Color

const
  Small = 6.cfloat
  Medium = 12.cfloat
  PagePadding = 16.cfloat
  ControlHeight = 34.cfloat
  RadiusSmall = 6.cfloat
  RadiusMedium = 9.cfloat
  MaxCardDepth = 16

var
  mode = Mode.dark
  layout: Layout
  cards: array[MaxCardDepth, CardState]
  cardDepth, activeField, nextId: int
  uiFont, boldFont: Font
  ownsUiFont, ownsBoldFont: bool

func rgb(r, g, b: uint8): Color = Color(r: r, g: g, b: b, a: 255)

func colors(): Palette =
  if mode == Mode.light:
    Palette(
      windowBg: rgb(250, 250, 250), viewBg: rgb(255, 255, 255),
      cardBg: rgb(255, 255, 255), accent: rgb(53, 132, 228),
      accentFg: rgb(255, 255, 255), accentHover: rgb(74, 147, 231),
      accentActive: rgb(28, 113, 216), destructive: rgb(192, 28, 40),
      success: rgb(38, 162, 105), text: rgb(36, 31, 49),
      textSecondary: rgb(94, 92, 100), textDisabled: rgb(154, 153, 150),
      borderSoft: rgb(224, 224, 224), buttonBg: rgb(224, 224, 224),
      buttonHover: rgb(208, 208, 208), buttonActive: rgb(192, 192, 192),
      buttonFg: rgb(36, 31, 49))
  else:
    Palette(
      windowBg: rgb(36, 36, 36), viewBg: rgb(30, 30, 30),
      cardBg: rgb(48, 48, 48), accent: rgb(53, 132, 228),
      accentFg: rgb(255, 255, 255), accentHover: rgb(74, 147, 231),
      accentActive: rgb(28, 113, 216), destructive: rgb(192, 28, 40),
      success: rgb(46, 194, 126), text: rgb(255, 255, 255),
      textSecondary: rgb(154, 153, 150), textDisabled: rgb(94, 92, 100),
      borderSoft: rgb(61, 56, 70), buttonBg: rgb(61, 61, 61),
      buttonHover: rgb(74, 74, 74), buttonActive: rgb(53, 53, 53),
      buttonFg: rgb(255, 255, 255))

proc loadUiFont(path: cstring; atlasSize: cint): bool =
  if path.isNil or not fileExists(path): return false
  let loaded = loadFontEx(path, (if atlasSize > 0: atlasSize else: 32), nil, 0)
  if not isFontValid(loaded) or loaded.texture.id == getFontDefault().texture.id:
    return false
  setTextureFilter(loaded.texture, TextureFilterBilinear)
  if ownsUiFont: unloadFont(uiFont)
  uiFont = loaded
  ownsUiFont = true
  true

proc loadPlatformFonts() =
  uiFont = getFontDefault()
  ownsUiFont = false
  for path in [
    "/usr/share/fonts/cantarell/Cantarell-VF.otf",
    "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/Library/Fonts/Arial Unicode.ttf"]:
    if loadUiFont(path.cstring, 32): break
  boldFont = uiFont
  ownsBoldFont = false
  for path in [
    "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"]:
    if fileExists(path.cstring):
      let loaded = loadFontEx(path.cstring, 32, nil, 0)
      if isFontValid(loaded) and loaded.texture.id != getFontDefault().texture.id:
        setTextureFilter(loaded.texture, TextureFilterBilinear)
        boldFont = loaded
        ownsBoldFont = true
        break

proc row(height: cfloat): Rectangle =
  result = Rectangle(x: layout.x, y: layout.y, width: layout.width, height: height)
  layout.y += height + Small

func roundness(rect: Rectangle; radius: cfloat): cfloat =
  min(1.cfloat, radius * 2 / min(rect.width, rect.height))

proc rounded(rect: Rectangle; radius: cfloat; fill, border: Color) =
  let amount = rect.roundness(radius)
  drawRectangleRounded(rect, amount, 8, fill)
  drawRectangleRoundedLinesEx(rect, amount, 8, 1, border)

proc hovered(rect: Rectangle): bool = checkCollisionPointRec(getMousePosition(), rect)

proc labelAt(font: Font; text: cstring; rect: Rectangle; size: cint; color: Color) =
  let value = if text.isNil: "".cstring else: text
  let measured = measureTextEx(font, value, size.cfloat, 0)
  drawTextEx(font, value, Vector2(x: rect.x, y: rect.y + (rect.height - measured.y) / 2),
             size.cfloat, 0, color)

proc textRole(font: Font; text: cstring; size: cint; color: Color; height: cfloat) =
  labelAt(font, text, row(height), size, color)

proc openWindow*(width = 900; height = 640; title = "Vidya"): bool {.discardable.} =
  if isWindowReady(): return false
  setConfigFlags(FlagWindowResizable or FlagVsyncHint or FlagMsaa4xHint)
  initWindow(width.cint, height.cint, title.cstring)
  if not isWindowReady(): return false
  loadPlatformFonts()
  setExitKey(KeyNull)
  true

proc closeWindow*() =
  if ownsUiFont: unloadFont(uiFont)
  if ownsBoldFont: unloadFont(boldFont)
  ownsUiFont = false
  ownsBoldFont = false
  if isWindowReady(): rlCloseWindow()

proc shouldClose*(): bool = windowShouldClose()
proc setTargetFps*(fps: int) = rlSetTargetFPS(fps.cint)
proc setMode*(value: Mode) = mode = value
proc currentMode*(): Mode = mode
proc loadFont*(path: string; atlasSize = 32): bool {.discardable.} =
  loadUiFont(path.cstring, atlasSize.cint)

proc beginFrame*() =
  beginDrawing()
  clearBackground(colors().windowBg)
  nextId = 0
  cardDepth = 0

proc endFrame*() = endDrawing()

proc pageBegin*(maxWidth = 0'f32) =
  let available = getScreenWidth().cfloat - PagePadding * 2
  layout.width = if maxWidth > 0: min(maxWidth.cfloat, available) else: available
  layout.x = (getScreenWidth().cfloat - layout.width) / 2
  layout.y = PagePadding

proc pageEnd*() = discard

proc cardBegin*() =
  if cardDepth >= MaxCardDepth: return
  let palette = colors()
  let parentFill = if cardDepth > 0: palette.cardBg else: palette.windowBg
  cards[cardDepth] = CardState(parent: layout, top: layout.y, parentFill: parentFill)
  inc cardDepth
  let surface = Rectangle(x: layout.x, y: layout.y, width: layout.width,
                          height: getScreenHeight().cfloat - layout.y)
  drawRectangleRounded(surface, surface.roundness(RadiusMedium), 8, palette.cardBg)
  layout.x += Medium
  layout.y += Medium
  layout.width -= Medium * 2

proc cardEnd*() =
  if cardDepth <= 0: return
  dec cardDepth
  let card = cards[cardDepth]
  let bottom = layout.y - Small + Medium
  let bounds = Rectangle(x: card.parent.x, y: card.top, width: card.parent.width,
                         height: bottom - card.top)
  let tail = Rectangle(x: card.parent.x, y: bottom, width: card.parent.width,
                       height: max(0.cfloat, getScreenHeight().cfloat - bottom))
  drawRectangleRec(tail, card.parentFill)
  drawRectangleRoundedLinesEx(bounds, bounds.roundness(RadiusMedium), 8, 1,
                              colors().borderSoft)
  layout = card.parent
  layout.y = bottom + Small

proc gap*(pixels: float32) = layout.y += pixels
proc separator*() = drawRectangleRec(row(1), colors().borderSoft)
proc title*(text: string) = textRole(boldFont, text.cstring, 20, colors().text, 26)
proc title2*(text: string) = textRole(boldFont, text.cstring, 16, colors().text, 22)
proc body*(text: string) = textRole(uiFont, text.cstring, 14, colors().text, 20)
proc dimLabel*(text: string) = textRole(uiFont, text.cstring, 12, colors().textSecondary, 18)

proc button*(label: string; kind = ButtonKind.normal): bool {.discardable.} =
  let palette = colors()
  let measured = measureTextEx(uiFont, label.cstring, 14, 0)
  let width = min(layout.width, max(112.cfloat, measured.x + Medium * 2))
  let fullRow = row(ControlHeight)
  let rect = Rectangle(x: fullRow.x, y: fullRow.y, width: width, height: fullRow.height)
  let over = hovered(rect)
  let down = over and isMouseButtonDown(MouseButtonLeft)
  var fill = if down: palette.buttonActive elif over: palette.buttonHover else: palette.buttonBg
  var foreground = palette.buttonFg
  var border = palette.borderSoft
  case kind
  of ButtonKind.primary:
    fill = if down: palette.accentActive elif over: palette.accentHover else: palette.accent
    foreground = palette.accentFg
    border = fill
  of ButtonKind.destructive:
    fill = palette.destructive
    foreground = rgb(255, 255, 255)
    border = fill
  of ButtonKind.normal: discard
  rounded(rect, RadiusMedium, fill, border)
  labelAt(uiFont, label.cstring,
          Rectangle(x: rect.x + (rect.width - measured.x) / 2, y: rect.y,
                    width: measured.x, height: rect.height), 14, foreground)
  over and isMouseButtonReleased(MouseButtonLeft)

proc primaryButton*(label: string): bool {.discardable.} = button(label, ButtonKind.primary)
proc destructiveButton*(label: string): bool {.discardable.} = button(label, ButtonKind.destructive)

proc checkbox*(label: string; checked: var bool): bool {.discardable.} =
  let palette = colors()
  let rect = row(ControlHeight)
  let box = Rectangle(x: rect.x, y: rect.y + 7, width: 20, height: 20)
  let over = hovered(rect)
  result = over and isMouseButtonReleased(MouseButtonLeft)
  if result: checked = not checked
  let fill = if checked: (if over: palette.accentHover else: palette.accent)
             elif over: palette.buttonHover else: palette.buttonBg
  rounded(box, RadiusSmall, fill, if checked: fill else: palette.borderSoft)
  if checked:
    drawLineEx(Vector2(x: box.x + 4, y: box.y + 10),
               Vector2(x: box.x + 8, y: box.y + 14), 2.2, palette.accentFg)
    drawLineEx(Vector2(x: box.x + 8, y: box.y + 14),
               Vector2(x: box.x + 16, y: box.y + 6), 2.2, palette.accentFg)
  labelAt(uiFont, label.cstring,
          Rectangle(x: rect.x + 30, y: rect.y, width: rect.width - 30, height: rect.height),
          14, palette.text)

proc status*(label: string; live: bool) =
  let palette = colors()
  let rect = row(20)
  let center = Vector2(x: rect.x + 5, y: rect.y + 10)
  let color = if live: palette.success else: palette.textSecondary
  if live: drawCircleV(center, 4, color) else: drawCircleLinesV(center, 4, color)
  labelAt(uiFont, label.cstring,
          Rectangle(x: rect.x + 16, y: rect.y, width: rect.width - 16, height: rect.height),
          14, palette.text)

proc editText(text: var string; capacity: int; rect: Rectangle): bool =
  let id = nextId + 1
  nextId = id
  if hovered(rect) and isMouseButtonPressed(MouseButtonLeft): activeField = id
  if activeField != id: return false
  var codepoint = getCharPressed()
  while codepoint > 0:
    let addition = $Rune(codepoint)
    if codepoint >= 32 and text.len + addition.len < capacity:
      text.add(addition)
      result = true
    codepoint = getCharPressed()
  if isKeyPressed(KeyBackspace) and text.len > 0:
    var newLen = text.len - 1
    while newLen > 0 and (text[newLen].ord and 0xc0) == 0x80:
      dec newLen
    text.setLen(newLen)
    result = true

proc textField*(text: var string; placeholder = ""; capacity = 1024): bool {.discardable.} =
  if capacity <= 0: raise newException(ValueError, "text field capacity must be positive")
  let palette = colors()
  let rect = row(ControlHeight)
  result = editText(text, max(capacity, text.len + 1), rect)
  let active = activeField == nextId
  rounded(rect, RadiusMedium, palette.viewBg,
          if active: palette.accent else: palette.borderSoft)
  let shown = if text.len > 0: text else: placeholder
  labelAt(uiFont, shown.cstring,
          Rectangle(x: rect.x + Medium, y: rect.y, width: rect.width - Medium * 2,
                    height: rect.height), 14,
          if text.len > 0: palette.text else: palette.textDisabled)

template page*(body: untyped) =
  pageBegin()
  try: body
  finally: pageEnd()

template page*(maxWidth: float32; body: untyped) =
  pageBegin(maxWidth)
  try: body
  finally: pageEnd()

template card*(body: untyped) =
  cardBegin()
  try: body
  finally: cardEnd()

proc run*(render: RenderProc; width = 900; height = 640; windowTitle = "Vidya") =
  if not openWindow(width, height, windowTitle):
    raise newException(IOError, "Vidya could not open a window")
  setTargetFps(60)
  try:
    while not shouldClose():
      beginFrame()
      try:
        page: render()
      finally: endFrame()
  finally: closeWindow()

# Stable C ABI. These adapters deliberately copy C strings at the boundary;
# all application-facing state remains ordinary, memory-safe Nim values.
proc vidya_open(width, height: cint; title: cstring): cint {.cdecl, exportc, dynlib.} =
  ord(openWindow(width.int, height.int, if title.isNil: "Vidya" else: $title)).cint
proc vidya_close() {.cdecl, exportc, dynlib.} = closeWindow()
proc vidya_should_close(): cint {.cdecl, exportc, dynlib.} = ord(shouldClose()).cint
proc vidya_set_target_fps(fps: cint) {.cdecl, exportc, dynlib.} = setTargetFps(fps.int)
proc vidya_set_mode(value: cint) {.cdecl, exportc, dynlib.} =
  setMode(if value == Mode.light.ord: Mode.light else: Mode.dark)
proc vidya_get_mode(): cint {.cdecl, exportc, dynlib.} = currentMode().ord.cint
proc vidya_load_font(path: cstring; atlasSize: cint): cint {.cdecl, exportc, dynlib.} =
  ord(not path.isNil and loadUiFont(path, atlasSize)).cint
proc vidya_begin_frame() {.cdecl, exportc, dynlib.} = beginFrame()
proc vidya_end_frame() {.cdecl, exportc, dynlib.} = endFrame()
proc vidya_page_begin(maxWidth: cfloat) {.cdecl, exportc, dynlib.} = pageBegin(maxWidth)
proc vidya_page_end() {.cdecl, exportc, dynlib.} = pageEnd()
proc vidya_card_begin() {.cdecl, exportc, dynlib.} = cardBegin()
proc vidya_card_end() {.cdecl, exportc, dynlib.} = cardEnd()
proc vidya_gap(pixels: cfloat) {.cdecl, exportc, dynlib.} = gap(pixels)
proc vidya_separator() {.cdecl, exportc, dynlib.} = separator()
proc vidya_title(text: cstring) {.cdecl, exportc, dynlib.} = title(if text.isNil: "" else: $text)
proc vidya_title_2(text: cstring) {.cdecl, exportc, dynlib.} = title2(if text.isNil: "" else: $text)
proc vidya_body(text: cstring) {.cdecl, exportc, dynlib.} = body(if text.isNil: "" else: $text)
proc vidya_dim_label(text: cstring) {.cdecl, exportc, dynlib.} = dimLabel(if text.isNil: "" else: $text)
proc vidya_button(label: cstring; kind: cint): cint {.cdecl, exportc, dynlib.} =
  let buttonKind = if kind == 1: ButtonKind.primary elif kind == 2: ButtonKind.destructive else: ButtonKind.normal
  ord(button(if label.isNil: "" else: $label, buttonKind)).cint
proc vidya_checkbox(label: cstring; checked: ptr cint): cint {.cdecl, exportc, dynlib.} =
  if checked.isNil: return 0
  var value = checked[] != 0
  result = ord(checkbox(if label.isNil: "" else: $label, value)).cint
  checked[] = ord(value).cint
proc vidya_checkbox_value(label: cstring; checked: cint): cint {.cdecl, exportc, dynlib.} =
  var value = checked != 0
  discard checkbox(if label.isNil: "" else: $label, value)
  ord(value).cint
proc vidya_status(label: cstring; live: cint) {.cdecl, exportc, dynlib.} =
  status(if label.isNil: "" else: $label, live != 0)
proc vidya_text_field(buffer: cstring; capacity: csize_t;
                      placeholder: cstring): cint {.cdecl, exportc, dynlib.} =
  if buffer.isNil or capacity == 0: return 0
  var value = $buffer
  let changed = textField(value, if placeholder.isNil: "" else: $placeholder, capacity.int)
  if changed:
    let count = min(value.len, capacity.int - 1)
    if count > 0: copyMem(buffer, unsafeAddr value[0], count)
    cast[ptr UncheckedArray[char]](buffer)[count] = '\0'
  ord(changed).cint
