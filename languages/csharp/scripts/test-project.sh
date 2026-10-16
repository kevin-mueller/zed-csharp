# Usage: DOTNET_FILE=<file> test-project.sh
. "$(dirname "$0")/lib.sh"
require_csproj "$DOTNET_FILE"
exec dotnet test "$csproj"
