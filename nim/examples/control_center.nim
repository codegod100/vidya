import vidya

var
  connected = false
  notifications = true
  startAtLogin = false
  usageData = false
  dark = true
  dirty = false

proc setting(label: string; value: var bool) =
  if checkbox(label, value):
    dirty = true

proc render() =
  title("Control Center")
  dimLabel("A stateful Nim app rendered by Vidya")
  gap(12)

  card:
    title2("Device")
    status(if connected: "Workstation connected" else: "Workstation offline",
           connected)
    body(if connected: "Preferences are ready to synchronize."
         else: "Connect this device to synchronize preferences.")
    if button(if connected: "Disconnect" else: "Connect device",
              if connected: ButtonKind.normal else: ButtonKind.primary):
      connected = not connected
      dirty = true

  gap(6)
  card:
    title2("Preferences")
    dimLabel("Changes remain local until you save them.")
    setting("Desktop notifications", notifications)
    setting("Start at login", startAtLogin)
    setting("Share anonymous usage data", usageData)

  gap(6)
  card:
    title2("Appearance")
    body(if dark: "Vidya dark is active." else: "Vidya light is active.")
    if button(if dark: "Use light theme" else: "Use dark theme"):
      dark = not dark
      setMode(if dark: Mode.dark else: Mode.light)
      dirty = true

  gap(6)
  card:
    title2("Apply changes")
    if dirty:
      status("Unsaved changes", false)
    else:
      dimLabel("No pending changes")
    if primaryButton("Save preferences"):
      dirty = false
    if destructiveButton("Reset to defaults"):
      connected = false
      notifications = true
      startAtLogin = false
      usageData = false
      dark = true
      dirty = false
      setMode(Mode.dark)

setMode(Mode.dark)
run(render, 680, 700, "Vidya Control Center")
