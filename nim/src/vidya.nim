## Nim bindings for Vidya's immediate-mode C ABI.
##
## The bindings load ``libvidya`` at runtime. Build either the Rust/egui or
## C/raylib implementation and put it on the platform's shared-library path.

const vidyaLibrary =
  when defined(windows): "vidya.dll"
  elif defined(macosx): "libvidya.dylib"
  else: "libvidya.so"

type
  Mode* {.pure.} = enum
    dark = 0
    light = 1

  ButtonKind* {.pure.} = enum
    normal = 0
    primary = 1
    destructive = 2

  RenderProc* = proc() {.closure.}

{.push cdecl, dynlib: vidyaLibrary.}
proc vidyaOpen(width, height: cint; title: cstring): cint {.importc: "vidya_open".}
proc vidyaClose() {.importc: "vidya_close".}
proc vidyaShouldClose(): cint {.importc: "vidya_should_close".}
proc vidyaSetTargetFps(fps: cint) {.importc: "vidya_set_target_fps".}
proc vidyaSetMode(mode: cint) {.importc: "vidya_set_mode".}
proc vidyaGetMode(): cint {.importc: "vidya_get_mode".}
proc vidyaLoadFont(path: cstring; atlasSize: cint): cint {.importc: "vidya_load_font".}
proc vidyaBeginFrame() {.importc: "vidya_begin_frame".}
proc vidyaEndFrame() {.importc: "vidya_end_frame".}
proc vidyaPageBegin(maxWidth: cfloat) {.importc: "vidya_page_begin".}
proc vidyaPageEnd() {.importc: "vidya_page_end".}
proc vidyaCardBegin() {.importc: "vidya_card_begin".}
proc vidyaCardEnd() {.importc: "vidya_card_end".}
proc vidyaGap(pixels: cfloat) {.importc: "vidya_gap".}
proc vidyaSeparator() {.importc: "vidya_separator".}
proc vidyaTitle(text: cstring) {.importc: "vidya_title".}
proc vidyaTitle2(text: cstring) {.importc: "vidya_title_2".}
proc vidyaBody(text: cstring) {.importc: "vidya_body".}
proc vidyaDimLabel(text: cstring) {.importc: "vidya_dim_label".}
proc vidyaButton(label: cstring; kind: cint): cint {.importc: "vidya_button".}
proc vidyaCheckbox(label: cstring; checked: ptr cint): cint {.importc: "vidya_checkbox".}
proc vidyaCheckboxValue(label: cstring; checked: cint): cint {.importc: "vidya_checkbox_value".}
proc vidyaStatus(label: cstring; live: cint) {.importc: "vidya_status".}
proc vidyaTextField(text: cstring; capacity: csize_t;
                    placeholder: cstring): cint {.importc: "vidya_text_field".}
{.pop.}

proc openWindow*(width = 900; height = 640;
                 title = "Vidya"): bool {.discardable.} =
  ## Open Vidya's single process-wide window.
  vidyaOpen(width.cint, height.cint, title.cstring) != 0

proc closeWindow*() = vidyaClose()
proc shouldClose*(): bool = vidyaShouldClose() != 0
proc setTargetFps*(fps: int) = vidyaSetTargetFps(fps.cint)
proc setMode*(mode: Mode) = vidyaSetMode(mode.ord.cint)
proc currentMode*(): Mode = Mode(vidyaGetMode())

proc loadFont*(path: string; atlasSize = 32): bool {.discardable.} =
  ## Replace the UI font. Call this between frames.
  vidyaLoadFont(path.cstring, atlasSize.cint) != 0

proc beginFrame*() = vidyaBeginFrame()
proc endFrame*() = vidyaEndFrame()
proc pageBegin*(maxWidth = 0'f32) = vidyaPageBegin(maxWidth.cfloat)
proc pageEnd*() = vidyaPageEnd()
proc cardBegin*() = vidyaCardBegin()
proc cardEnd*() = vidyaCardEnd()
proc gap*(pixels: float32) = vidyaGap(pixels.cfloat)
proc separator*() = vidyaSeparator()

proc title*(text: string) = vidyaTitle(text.cstring)
proc title2*(text: string) = vidyaTitle2(text.cstring)
proc body*(text: string) = vidyaBody(text.cstring)
proc dimLabel*(text: string) = vidyaDimLabel(text.cstring)

proc button*(label: string; kind = ButtonKind.normal): bool {.discardable.} =
  vidyaButton(label.cstring, kind.ord.cint) != 0

proc primaryButton*(label: string): bool {.discardable.} =
  button(label, ButtonKind.primary)

proc destructiveButton*(label: string): bool {.discardable.} =
  button(label, ButtonKind.destructive)

proc checkbox*(label: string; checked: var bool): bool {.discardable.} =
  ## Draw a checkbox, update ``checked``, and report whether it changed.
  var value = ord(checked).cint
  result = vidyaCheckbox(label.cstring, addr value) != 0
  checked = value != 0

proc status*(label: string; live: bool) =
  vidyaStatus(label.cstring, ord(live).cint)

proc textField*(text: var string; placeholder = "";
                capacity = 1024): bool {.discardable.} =
  ## Draw a single-line field, updating ``text`` when it changes.
  ## ``capacity`` includes the terminating NUL byte.
  if capacity <= 0:
    raise newException(ValueError, "text field capacity must be positive")
  let bufferSize = max(capacity, text.len + 1)
  var buffer = newString(bufferSize)
  if text.len > 0:
    copyMem(addr buffer[0], unsafeAddr text[0], text.len)
  buffer[text.len] = '\0'
  result = vidyaTextField(buffer.cstring, bufferSize.csize_t,
                          placeholder.cstring) != 0
  text = $buffer.cstring

template page*(body: untyped) =
  ## Render ``body`` in a full-width page and always restore its parent.
  pageBegin()
  try:
    body
  finally:
    pageEnd()

template page*(maxWidth: float32; body: untyped) =
  ## Render ``body`` in a width-limited page.
  pageBegin(maxWidth)
  try:
    body
  finally:
    pageEnd()

template card*(body: untyped) =
  ## Render ``body`` in a card and always restore its parent.
  cardBegin()
  try:
    body
  finally:
    cardEnd()

proc run*(render: RenderProc; width = 900; height = 640;
          windowTitle = "Vidya") =
  ## Own the window loop and invoke ``render`` once per frame.
  if not openWindow(width, height, windowTitle):
    raise newException(IOError, "Vidya could not open a window")
  setTargetFps(60)
  try:
    while not shouldClose():
      beginFrame()
      try:
        page:
          render()
      finally:
        endFrame()
  finally:
    closeWindow()
