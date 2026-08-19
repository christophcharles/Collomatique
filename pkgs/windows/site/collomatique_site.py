"""Collomatique's own directory for Python modules a user installs.

Run by ``collomatique.pth`` every time this interpreter starts, whether that is
the application, ``python.exe`` or pip, so all three agree about what is
installed. That is the whole reason this is a .pth file and not an environment
variable, which would reach only the application.

``collomatique-pip.cmd`` asks this module where to install, which is why the
path is written here and nowhere else.

The Python version is part of the path, as it is in every Python installation:
a compiled module built for 3.12 cannot be loaded by 3.13, so a new interpreter
has to start with an empty directory rather than inherit files it cannot use.
The previous one is left where it is, and its ``*.dist-info`` folders name
everything that was installed -- which is what a future version can read to
offer putting them back.

``%APPDATA%\\collomatique`` rather than a directory of Python's own: this is
beside the ``config`` directory the application already creates there, and it is
not shared with any other Python of the same version on the machine.

``Lib\\site-packages`` under the version is the layout ``pip install --prefix``
produces on Windows. Matching pip's own scheme is what keeps ``pip list`` and
``pip uninstall`` working afterwards.
"""

import os
import site
import sys


def prefix():
    """The install prefix, or None when there is no profile to put it in."""
    appdata = os.environ.get("APPDATA")
    if not appdata:
        return None
    version = f"{sys.version_info.major}.{sys.version_info.minor}"
    return os.path.join(appdata, "collomatique", "python", version)


_prefix = prefix()
if _prefix:
    # addsitedir rather than sys.path.append: it makes this a real site
    # directory, so .pth files inside it are processed like any other package
    # install. It ignores a directory that does not exist, so nothing has to be
    # created before the first module is installed.
    site.addsitedir(os.path.join(_prefix, "Lib", "site-packages"))
