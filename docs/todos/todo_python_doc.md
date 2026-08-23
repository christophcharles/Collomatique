# TODO: Document the Python API

The Python API should be **entirely** documented. Two things are needed, not
one:

- **API reference** — every exposed class, method and field.
- **A "where to start" guide** — prose that walks a newcomer through opening a
  file, reading data and making a first modification.

A likely toolchain is Sphinx + MyST (Markdown rather than reST) with autodoc to
pull the reference straight from the docstrings.

The API itself is done — its design is `docs/python/new_api_design.md`, which is
where the reasoning behind every door lives and so where a writer should start.

One open question: the module ships **no `.pyi` type stubs**, and nothing
generates them. The design asked for them (§7) and they never landed. Whether
they are worth writing — for autodoc, for editors, for type checkers — is
undecided.
