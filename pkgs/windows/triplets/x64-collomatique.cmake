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

# The COIN-OR libraries, and only those, are built static.
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
# The rest of CBC's tree -- zlib, bzip2, lapack -- stays dynamic. No data
# symbols are read from those.
#
# coinutils carries no coin-or- prefix, hence the second test.
if(PORT MATCHES "^coin-or-" OR PORT STREQUAL "coinutils")
    set(VCPKG_LIBRARY_LINKAGE static)
endif()
