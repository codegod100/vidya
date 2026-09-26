import vidya

var enabled = true

proc render() =
  title("Vidya")
  dimLabel("Nim → C ABI → Vidya")
  gap(12)
  card:
    title2("Actions")
    body("The application remains ordinary Nim code.")
    if primaryButton("Primary action"):
      echo "primary action"
    button("Default action")
    destructiveButton("Destructive action")
    discard checkbox("Sync preferences", enabled)
    status(if enabled: "Connected" else: "Offline", enabled)

setMode(Mode.dark)
run(render, 720, 480, "Vidya · Nim")
