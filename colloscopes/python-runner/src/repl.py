import builtins
import code
import os
import sys

import collomatique


def run(read_line):
    # The transcript is not a terminal, but the worker's output is a pty and
    # every isatty() check says otherwise. Left alone, help() hands its text to
    # `less`, which writes escape sequences and then waits for a keypress that
    # can never arrive. This is the environment pydoc reads to answer "plain
    # text, please".
    os.environ.pop("PAGER", None)
    os.environ.pop("MANPAGER", None)
    os.environ["TERM"] = "dumb"

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
