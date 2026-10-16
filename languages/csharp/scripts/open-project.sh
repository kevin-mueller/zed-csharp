# Usage: DOTNET_FILE=<file> open-project.sh
# Opens the nearest .csproj with the CLI of the Zed channel the task runs in.
. "$(dirname "$0")/lib.sh"
require_csproj "$DOTNET_FILE"

case "$TERM_PROGRAM_VERSION" in
    *preview*) candidates="zed-preview zed zeditor" ;;
    *) candidates="zed zeditor zed-preview" ;;
esac

for cli in $candidates; do
    if command -v "$cli" >/dev/null 2>&1; then
        exec "$cli" "$csproj"
    fi
done

echo "No Zed CLI found (tried: $candidates). Install it via 'cli: install' in the command palette." >&2
exit 1
