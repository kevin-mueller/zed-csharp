# Usage: DOTNET_FILE=<file> build-project.sh
. "$(dirname "$0")/lib.sh"
require_csproj "$DOTNET_FILE"
exec dotnet build "$csproj"
