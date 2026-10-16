# Shared helpers for the task scripts. Sourced, not executed.
# All scripts are POSIX sh, so they work regardless of the user's shell. They take their
# input from env vars (DOTNET_FILE, ...) because Zed passes task args to the shell unquoted.

# Prints the nearest .csproj, searching upwards from the given file or directory.
find_csproj() {
    case "$1" in
        *.csproj) printf '%s\n' "$1"; return 0 ;;
    esac

    dir=$1
    [ -d "$dir" ] || dir=$(dirname "$dir")
    while :; do
        csproj=$(find "$dir" -maxdepth 1 -name '*.csproj' -print -quit 2>/dev/null)
        if [ -n "$csproj" ]; then
            printf '%s\n' "$csproj"
            return 0
        fi
        case "$dir" in
            / | .) return 1 ;;
        esac
        dir=$(dirname "$dir")
    done
}

# Sets $csproj to the nearest .csproj of the given path, or exits with an error.
require_csproj() {
    csproj=$(find_csproj "$1") || {
        echo "No .csproj found for $1" >&2
        exit 1
    }
}
