# Usage: DOTNET_FILE=<file> run-project.sh
. "$(dirname "$0")/lib.sh"
require_csproj "$DOTNET_FILE"
exec dotnet run --project "$csproj"
