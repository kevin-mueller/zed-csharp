; Razor blocks: @code { }, @functions { }, @{ }
(razor_block
  "{" @fold
  "}" @fold.end)

; Razor control flow blocks
(razor_if
  "{" @fold
  "}" @fold.end)

(razor_else
  "{" @fold
  "}" @fold.end)

(razor_else_if
  "{" @fold
  "}" @fold.end)

(razor_for
  "{" @fold
  "}" @fold.end)

(razor_foreach
  "{" @fold
  "}" @fold.end)

(razor_while
  "{" @fold
  "}" @fold.end)

(razor_do_while
  "{" @fold
  "}" @fold.end)

(razor_try
  "{" @fold
  "}" @fold.end)

(razor_catch
  "{" @fold
  "}" @fold.end)

(razor_finally
  "{" @fold
  "}" @fold.end)

(razor_section
  "{" @fold
  "}" @fold.end)

; HTML elements
(element
  "<" @fold
  ">" @fold.end)

; Razor comments
(razor_comment) @fold

; HTML comments
(html_comment) @fold
