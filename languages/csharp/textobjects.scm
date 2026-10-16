; comments
(comment)+ @comment.around

; functions
(method_declaration) @function.around

(method_declaration
  body: (block
    "{"
    (_)* @function.inside
    "}"))

(method_declaration
  body: (arrow_expression_clause
    (_) @function.inside))

(constructor_declaration
  body: (block
    "{"
    (_)* @function.inside
    "}")) @function.around

(constructor_declaration
  body: (arrow_expression_clause
    (_) @function.inside)) @function.around

(destructor_declaration
  body: (block
    "{"
    (_)* @function.inside
    "}")) @function.around

(operator_declaration
  body: (block
    "{"
    (_)* @function.inside
    "}")) @function.around

(conversion_operator_declaration
  body: (block
    "{"
    (_)* @function.inside
    "}")) @function.around

(local_function_statement
  body: (block
    "{"
    (_)* @function.inside
    "}")) @function.around

(local_function_statement
  body: (arrow_expression_clause
    (_) @function.inside)) @function.around

(accessor_declaration
  body: (block
    "{"
    (_)* @function.inside
    "}")) @function.around

(lambda_expression
  body: (block
    "{"
    (_)* @function.inside
    "}")) @function.around

(lambda_expression
  body: (_) @function.inside
  (#not-match? @function.inside "^\\{")) @function.around

; classes
(class_declaration
  body: (_
    "{"
    (_)* @class.inside
    "}")) @class.around

(struct_declaration
  body: (_
    "{"
    (_)* @class.inside
    "}")) @class.around

(record_declaration
  body: (_
    "{"
    (_)* @class.inside
    "}")) @class.around

(record_declaration
  !body) @class.around

(interface_declaration
  body: (_
    "{"
    (_)* @class.inside
    "}")) @class.around

(enum_declaration
  body: (_
    "{"
    [
      (_)
      ","?
    ]* @class.inside
    "}")) @class.around

(extension_declaration
  (extension_body
    "{"
    (_)* @class.inside
    "}")) @class.around
