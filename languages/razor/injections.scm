; inherits: c_sharp

; Comments as comment language
([
  (html_comment)
  (razor_comment)
] @injection.content
  (#set! injection.language "comment"))

; HTML elements get HTML highlighting
((element) @injection.content
  (#set! injection.language "html"))
