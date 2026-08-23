# TODO: Document the Python API

The Python API should be **entirely** documented. Two things are needed, not
one:

- **API reference** — every exposed class, method and field.
- **A "where to start" guide** — prose that walks a newcomer through opening a
  file, reading data and making a first modification.

A likely toolchain is Sphinx + MyST (Markdown rather than reST) with autodoc to
pull the reference straight from the docstrings.

This should follow [the Python API revamp](todo_python_api.md): documenting the
current API before it is reworked would be wasted effort.
