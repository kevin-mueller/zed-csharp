; inherits: c_sharp

; ============================================================
; Razor directive markers (@page, @using, @model, @code, …)
; ============================================================

[
  "at_page"
  "at_using"
  "at_model"
  "at_rendermode"
  "at_inject"
  "at_implements"
  "at_layout"
  "at_inherits"
  "at_attribute"
  "at_typeparam"
  "at_namespace"
  "at_preservewhitespace"
  "at_block"
  "at_at_escape"
  "at_colon_transition"
] @keyword.directive

[
  "at_lock"
  "at_section"
] @keyword

[
  "at_if"
  "at_switch"
] @keyword.conditional

[
  "at_for"
  "at_foreach"
  "at_while"
  "at_do"
] @keyword.repeat

[
  "at_try"
] @keyword.exception

; ============================================================
; Razor expressions — @ markers only, not the whole expression
; ============================================================

[
  "at_implicit"
  "at_explicit"
] @punctuation.special

(razor_implicit_expression
  "at_implicit" @keyword
  (await_expression
    "await" @keyword))

; ============================================================
; Razor render mode values
; ============================================================

(razor_rendermode) @constant

; ============================================================
; Blazor event / bind attributes  (@onclick, @bind, @ref …)
; ============================================================

(razor_attribute_name) @attribute
(razor_attribute_modifier) @attribute

; ============================================================
; Comments
; ============================================================

[
  (razor_comment)
  (html_comment)
] @comment

; ============================================================
; Explicit C# highlights for @code / @functions blocks
; (duplicated from c_sharp highlights.scm as a reliable fallback
;  in case the ; inherits directive is not resolved by the host)
; ============================================================

(identifier) @variable

(method_declaration name: (identifier) @function)
(_ function: (identifier) @function)
(local_function_statement name: (identifier) @function)
(invocation_expression
  (member_access_expression name: (identifier) @function))

(interface_declaration name: (identifier) @type)
(class_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(struct_declaration (identifier) @type)
(record_declaration (identifier) @type)
(namespace_declaration name: (identifier) @type)

(generic_name (identifier) @type)
(type_parameter (identifier) @property.definition)
(parameter type: (identifier) @type)
(type_argument_list (identifier) @type)
(as_expression right: (identifier) @type)
(is_expression right: (identifier) @type)
(_ type: (identifier) @type)
(base_list (identifier) @type)

(constructor_declaration name: (identifier) @constructor)
(destructor_declaration name: (identifier) @constructor)

(predefined_type) @type.builtin

(enum_member_declaration (identifier) @property.definition)

(property_declaration name: (identifier) @property)

(parameter name: (identifier) @variable.parameter)

(attribute name: (identifier) @attribute)

(type_parameter_constraints_clause (identifier) @property.definition)

[
  (real_literal)
  (integer_literal)
] @number

[
  (character_literal)
  (string_literal)
  (raw_string_literal)
  (verbatim_string_literal)
  (interpolated_string_expression)
  (interpolation_start)
  (interpolation_quote)
] @string

(escape_sequence) @string.escape

[
  (boolean_literal)
  (null_literal)
] @constant.builtin

(comment) @comment

[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  (interpolation_brace)
] @punctuation.bracket

[
  "--"
  "-"
  "-="
  "&"
  "&="
  "&&"
  "+"
  "++"
  "+="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "!"
  "!="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "|"
  "|="
  "||"
  "?"
  "??"
  "??="
  "^"
  "^="
  "~"
  "*"
  "*="
  "/"
  "/="
  "%"
  "%="
  ":"
] @operator

[
  (modifier)
  "this"
  (implicit_type)
] @keyword

[
  "class"
  "struct"
  "interface"
  "enum"
  "record"
  "delegate"
] @keyword.type

[
  "if"
  "else"
  "switch"
  "case"
  "default"
  "when"
] @keyword.conditional

[
  "for"
  "foreach"
  "while"
  "do"
  "in"
] @keyword.repeat

[
  "try"
  "catch"
  "finally"
  "throw"
] @keyword.exception

[
  "return"
  "break"
  "continue"
  "goto"
  "yield"
] @keyword.return

[
  "using"
  "namespace"
  "global"
] @keyword.import

[
  "async"
  "await"
] @keyword.coroutine

[
  "new"
  "sizeof"
  "stackalloc"
  "typeof"
  "is"
  "as"
  "with"
] @keyword.operator

[
  "add"
  "alias"
  "base"
  "checked"
  "event"
  "explicit"
  "extern"
  "from"
  "get"
  "implicit"
  "init"
  "let"
  "lock"
  "notnull"
  "operator"
  "out"
  "params"
  "ref"
  "remove"
  "select"
  "set"
  "static"
  "unchecked"
  "where"
] @keyword
