# FIXME: Pairing rules should not display (or even support) subjects without interrogations

The gtk4 GUI and the backend code accepts subjects that do not have interrogations (in parameters, so as part of the subject definition).
This basically leads to impossible constraints or trivial ones. It should be forbidden (both in the backend and in the GUI).
