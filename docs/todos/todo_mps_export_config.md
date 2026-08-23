# TODO: Make the MPS export configurable in the GUI

Like the Python API, the MPS export in the GUI should let the user choose *what*
model is exported. Today "Exporter le problème ILP (MPS)" in the "Outils
avancés" panel exports a fixed model.

The configuration is the same one the solver already asks for, so the solve
configuration dialog (`gtk4/src/editor/run_solver/conductor_config.rs`) should
be made generic — or at least shared — and the MPS export should call it to pick
the model before writing the file.
