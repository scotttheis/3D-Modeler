#!/bin/sh
# CMake dependencies such as manifold-csg-sys select the host architecture
# unless their C++ compiler itself targets Intel on Apple Silicon.
for argument in "$@"; do
    case "$argument" in
        */_deps/tbb-src/*)
            # CMake sees the Apple Silicon host while building this Intel
            # target, so oneTBB misses the feature flags it normally enables
            # for x86_64. Match its intended x86 compile flags here.
            exec /usr/bin/clang++ -arch x86_64 -mrtm -mwaitpkg "$@"
            ;;
    esac
done
exec /usr/bin/clang++ -arch x86_64 "$@"
