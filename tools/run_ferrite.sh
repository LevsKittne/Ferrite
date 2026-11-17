#!/bin/sh
set -e

dir="$(cd "$(dirname "$0")" && pwd)"
executable="${FERRITE_GAME:-}"

if [ -n "$1" ] && [ -e "$1" ] && [ -z "$executable" ]; then
    executable="$1"
    shift
fi

case "$(uname)" in
    Darwin)
        lib="$dir/libferrite.dylib"
        [ -f "$lib" ] || lib="$dir/libversion.dylib"
        if [ -z "$executable" ]; then
            for app in "$dir"/*.app; do
                [ -d "$app" ] && executable="$app" && break
            done
        fi
        case "$executable" in
            *.app|*.app/)
                app="${executable%/}"
                name="$(defaults read "$app/Contents/Info" CFBundleExecutable)"
                executable="$app/Contents/MacOS/$name"
                ;;
        esac
        export DYLD_INSERT_LIBRARIES="$lib${DYLD_INSERT_LIBRARIES:+:$DYLD_INSERT_LIBRARIES}"
        ;;
    *)
        lib="$dir/libferrite.so"
        [ -f "$lib" ] || lib="$dir/libversion.so"
        if [ -z "$executable" ]; then
            for candidate in "$dir"/*.x86_64 "$dir"/*.x86; do
                [ -f "$candidate" ] && executable="$candidate" && break
            done
        fi
        export LD_PRELOAD="$lib${LD_PRELOAD:+:$LD_PRELOAD}"
        ;;
esac

if [ -z "$executable" ]; then
    echo "run_ferrite.sh: game executable not found, set FERRITE_GAME or pass it as the first argument" >&2
    exit 1
fi
if [ ! -f "$lib" ]; then
    echo "run_ferrite.sh: $lib not found" >&2
    exit 1
fi

export FERRITE_DIR="${FERRITE_DIR:-$dir/Ferrite}"
exec "$executable" "$@"
