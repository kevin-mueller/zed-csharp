# Usage: DOTNET_FILE=<file> CSHARP_TEST_CLASS=<class> [CSHARP_TEST_METHOD=<method>] run-tests.sh
# Runs a single test class or method. TUnit projects are run via Microsoft.Testing.Platform,
# everything else via `dotnet test`.
. "$(dirname "$0")/lib.sh"
require_csproj "$DOTNET_FILE"
class=$CSHARP_TEST_CLASS
method=$CSHARP_TEST_METHOD

if grep -qs TUnit "$csproj"; then
    exec dotnet run --project "$csproj" -- --treenode-filter "/*/*/$class/${method:-*}"
elif [ -n "$method" ]; then
    exec dotnet test "$csproj" --filter "FullyQualifiedName~$class.$method"
else
    exec dotnet test "$csproj" --filter "FullyQualifiedName~$class"
fi
