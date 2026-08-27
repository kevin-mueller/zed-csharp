(identifier) @variable

;; Methods

(method_declaration name: (identifier) @function)
(_ function: (identifier) @function)
(local_function_statement name: (identifier) @function)

;; Types

(interface_declaration name: (identifier) @type)
(class_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(struct_declaration (identifier) @type)
(record_declaration (identifier) @type)
(namespace_declaration name: (identifier) @module)
(file_scoped_namespace_declaration name: (identifier) @module)

(generic_name (identifier) @type)
(type_parameter (identifier) @property.definition)
(parameter type: (identifier) @type)
(type_argument_list (identifier) @type)
(as_expression right: (identifier) @type)
(is_expression right: (identifier) @type)

(constructor_declaration name: (identifier) @constructor)
(destructor_declaration name: (identifier) @constructor)

(_ type: (identifier) @type)

(base_list (identifier) @type)

(predefined_type) @type.builtin

;; Enum

(enum_member_declaration (identifier) @property.definition)

;; Properties

(property_declaration name: (identifier) @property)

;; Literals

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

;; Comments

(comment) @comment

;; Tokens

[
  ";"
  "."
  ","
] @punctuation.delimiter

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
  ".."
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  (interpolation_brace)
] @punctuation.bracket

;; Keywords — modifiers / misc

[
  (modifier)
  "this"
  (implicit_type)
] @keyword

;; Keywords — type declarations

[
  "class"
  "struct"
  "interface"
  "enum"
  "record"
  "delegate"
] @keyword.type

;; Keywords — control flow: conditional

[
  "if"
  "else"
  "switch"
  "case"
  "default"
  "when"
] @keyword.conditional

;; Keywords — control flow: loops

[
  "for"
  "foreach"
  "while"
  "do"
  "in"
] @keyword.repeat

;; Keywords — control flow: exception handling

[
  "try"
  "catch"
  "finally"
  "throw"
] @keyword.exception

;; Keywords — control flow: return / jump

[
  "return"
  "break"
  "continue"
  "goto"
  "yield"
] @keyword.return

;; Keywords — imports / namespace

[
  "using"
  "namespace"
  "global"
] @keyword.import

;; Keywords — async

[
  "async"
  "await"
] @keyword.coroutine

;; Keywords — operators / allocation

[
  "new"
  "sizeof"
  "stackalloc"
  "typeof"
  "is"
  "as"
  "with"
] @keyword.operator

;; Keywords — misc

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

;; Attribute

(attribute name: (identifier) @attribute)

;; Parameters

(parameter
  name: (identifier) @variable.parameter)

;; Type constraints

(type_parameter_constraints_clause (identifier) @property.definition)

;; Method calls

(invocation_expression (member_access_expression name: (identifier) @function))

; pattern expression keywords
(negated_pattern
  "not" @keyword)

(and_pattern
  "and" @keyword)

(or_pattern
  "or" @keyword)

; scoped keyword
(scoped_type
  "scoped" @keyword)
