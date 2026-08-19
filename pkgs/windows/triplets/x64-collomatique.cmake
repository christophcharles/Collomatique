# The triplet the Windows build of Collomatique is compiled with.
#
# It is x64-windows with two changes: release only, and the COIN-OR libraries
# built static. Used through --overlay-triplets, see build.ps1.

set(VCPKG_TARGET_ARCHITECTURE x64)
set(VCPKG_CRT_LINKAGE dynamic)
set(VCPKG_LIBRARY_LINKAGE dynamic)

# Release only. We ship release, and this halves a build measured in hours.
# A debug cargo build against these libraries is still a consistent link:
# rustc links the release CRT whatever the profile.
set(VCPKG_BUILD_TYPE release)

# The COIN-OR libraries and the linear algebra they sit on are built static.
#
# collo-cbc/cpp/collo_cbc.cpp declares
#
#     extern CglPreProcess* cbcPreProcessPointer;
#
# which is a *data* symbol. An import library only offers __imp_ for data, and
# MSVC's linker will not synthesise the indirection the way mingw's does, so a
# plain extern like that one resolves against a static library and not against a
# DLL. Linking these static needs no change to the C++ at all; the other route,
# annotating the declaration __declspec(dllimport) under _MSC_VER, would tie
# that source file to this one with nothing checking that they agree.
#
# zlib and bzip2 stay dynamic. No data symbols are read from those.
#
# The linear algebra ports are in the list for a different reason, and it is not
# about symbols at all. vcpkg's `lapack` is a metapackage that picks its provider
# from a platform expression:
#
#     clapack           when  ... (uwp | (arm & windows) | (static & windows & !mingw))
#     lapack-reference  when  ... !uwp & !(arm & windows) & !(static & windows & !mingw)
#
# where `static` means VCPKG_LIBRARY_LINKAGE. Left dynamic, that resolves to
# lapack-reference, which on Windows pulls in vcpkg-gfortran and fails to build
# (microsoft/vcpkg#49688, open and stale since January). Built static it resolves
# to clapack, the f2c translation, whose own dependency openblas needs no Fortran
# compiler. So this does not repair lapack-reference; it takes it out of the
# graph. clapack, blas and openblas follow coinutils rather than being linked
# against it as DLLs, which also keeps CBC and its numerics one static blob with
# nothing extra to ship.
#
# coinutils carries no coin-or- prefix, hence the explicit tests.
if(PORT MATCHES "^coin-or-"     OR
   PORT STREQUAL "coinutils"    OR
   PORT STREQUAL "lapack"       OR
   PORT STREQUAL "clapack"      OR
   PORT STREQUAL "blas"         OR
   PORT STREQUAL "openblas")
    set(VCPKG_LIBRARY_LINKAGE static)
endif()
