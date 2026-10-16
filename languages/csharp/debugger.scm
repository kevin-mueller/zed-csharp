; Declarations
(parameter
  name: (identifier) @debug-variable)

(variable_declarator
  name: (identifier) @debug-variable)

(foreach_statement
  left: (identifier) @debug-variable)

(catch_declaration
  name: (identifier) @debug-variable)

(declaration_pattern
  name: (identifier) @debug-variable)

; Usages
(assignment_expression
  left: (identifier) @debug-variable)

(member_access_expression
  expression: (identifier) @debug-variable
  (#not-match? @debug-variable "^[A-Z]"))

(element_access_expression
  expression: (identifier) @debug-variable)

(binary_expression
  (identifier) @debug-variable
  (#not-match? @debug-variable "^[A-Z]"))

(argument
  !name
  (identifier) @debug-variable
  (#not-match? @debug-variable "^[A-Z]"))

(argument
  name: (_)
  (identifier) @debug-variable
  (#not-match? @debug-variable "^[A-Z]"))

(return_statement
  (identifier) @debug-variable)

(yield_statement
  (identifier) @debug-variable)

(await_expression
  (identifier) @debug-variable)

(prefix_unary_expression
  (identifier) @debug-variable)

(postfix_unary_expression
  (identifier) @debug-variable)

(parenthesized_expression
  (identifier) @debug-variable)

(conditional_expression
  (identifier) @debug-variable)

(is_pattern_expression
  expression: (identifier) @debug-variable)

(if_statement
  condition: (identifier) @debug-variable)

(while_statement
  condition: (identifier) @debug-variable)

(switch_statement
  value: (identifier) @debug-variable)

(switch_expression
  .
  (identifier) @debug-variable)

(interpolation
  (identifier) @debug-variable)

(initializer_expression
  (identifier) @debug-variable)

(arrow_expression_clause
  (identifier) @debug-variable)

(block) @debug-scope
