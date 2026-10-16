# Usage: DOTNET_FILE=<file> new-type.sh
# Writes a type declaration named after the file into an empty .cs file. The namespace is
# derived from the nearest .csproj's RootNamespace (or its file name) plus the folder path,
# like Roslyn does. Files named like IFoo.cs get an interface, all others a class.
. "$(dirname "$0")/lib.sh"
file=$DOTNET_FILE

if [ -s "$file" ]; then
    echo "$file is not empty, not overwriting it." >&2
    exit 1
fi

name=$(basename "$file" .cs)
dir=$(dirname "$file")

namespace=""
if csproj=$(find_csproj "$dir"); then
    project_dir=$(dirname "$csproj")
    namespace=$(sed -n 's:.*<RootNamespace>\(.*\)</RootNamespace>.*:\1:p' "$csproj" | head -n 1)
    [ -n "$namespace" ] || namespace=$(basename "$csproj" .csproj)
    relative=$(awk -v a="$dir" -v b="$project_dir" 'BEGIN { print substr(a, length(b) + 2) }')
    [ -n "$relative" ] && namespace="$namespace.$relative"
    namespace=$(printf '%s' "$namespace" | sed 's:/:.:g; s:[ -]:_:g')
fi

kind=class
printf '%s' "$name" | grep -qE '^I[A-Z]' && kind=interface

{
    [ -n "$namespace" ] && printf 'namespace %s;\n\n' "$namespace"
    printf 'public %s %s\n{\n}\n' "$kind" "$name"
} >"$file"
