# TODO: Python's text encoding on Windows

Text comes out wrong when Python runs on Windows. It shows in the **Python
console**, which is where it was seen, but nothing says the console is the only
place: a script that prints, or that opens a file without naming an encoding, is
on the same footing.

What the code says, none of it checked against a running Windows build:

- The worker's output travels as **bytes over a pty** — ConPTY on Windows — and
  becomes text through `String::from_utf8_lossy`. Whatever the interpreter wrote
  in another encoding arrives as replacement characters rather than as an error.
- Nothing in the tree sets `PYTHONUTF8` or `PYTHONIOENCODING`, and nothing
  reconfigures `sys.stdout`. The embedded interpreter therefore picks its own
  default, which on Windows is the ANSI code page and not UTF-8.
- The console's banner and the module's messages are French, so accents are on
  the path from the very first line printed.

First step is to pin the symptom down: which characters, in which direction
(printed output, a typed line, `input()`, a traceback), and whether anything
raises `UnicodeEncodeError` instead of quietly printing the wrong thing.

The fix is then probably UTF-8 mode on the interpreter, or `sys.stdout` and
`sys.stderr` reconfigured where the interpreter is started. Whichever it is, it
belongs to the worker and so to every script, not to the console alone.
