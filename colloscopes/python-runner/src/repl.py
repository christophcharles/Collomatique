import builtins
import code
import sys

import collomatique


def run(read_line):
    def _input(prompt=""):
        _flush()
        return read_line(str(prompt))

    def _flush():
        sys.stdout.flush()
        sys.stderr.flush()

    # input() in user code is asked for the same way as the console's own prompt.
    builtins.input = _input

    class _Console(code.InteractiveConsole):
        def raw_input(self, prompt=""):
            # Whatever was printed belongs before the prompt in the transcript.
            _flush()
            return read_line(str(prompt))

    console = _Console(
        locals={
            "__name__": "__console__",
            "__doc__": None,
            "collomatique": collomatique,
            "clm": collomatique,
        }
    )
    banner = (
        f"Python {sys.version.split()[0]} — collomatique {collomatique.__version__}\n"
        "Le module « collomatique » est déjà importé (alias « clm »)."
    )
    try:
        console.interact(banner=banner, exitmsg="Session terminée.")
    except SystemExit:
        # exit()/quit(): a clean ending for the console, not an error.
        pass
    finally:
        _flush()
