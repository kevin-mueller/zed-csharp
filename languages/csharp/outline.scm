(comment) @annotation

(namespace_declaration
    "namespace" @context
    name: (_) @name
) @item

(file_scoped_namespace_declaration
    "namespace" @context
    name: (_) @name
) @item

(class_declaration
    "class" @context
    name: (identifier) @name
) @item

(struct_declaration
    "struct" @context
    name: (identifier) @name
) @item

(record_declaration
    "record" @context
    name: (identifier) @name
) @item

(interface_declaration
    "interface" @context
    name: (identifier) @name
) @item

(enum_declaration
    "enum" @context
    name: (identifier) @name
) @item

(enum_member_declaration
    name: (identifier) @name
) @item

(extension_declaration
    "extension" @context
    (receiver_parameter
        type: (_) @name)
) @item

(delegate_declaration
    "delegate" @context
    name: (identifier) @name
    parameters: (parameter_list) @context
) @item

(constructor_declaration
    name: (identifier) @name
    parameters: (parameter_list) @context
) @item

(destructor_declaration
    "~" @context
    name: (identifier) @name
) @item

(method_declaration
    name: (identifier) @name
    parameters: (parameter_list) @context
) @item

(local_function_statement
    name: (identifier) @name
    parameters: (parameter_list) @context
) @item

(operator_declaration
    "operator" @context
    operator: _ @name
    parameters: (parameter_list) @context
) @item

(conversion_operator_declaration
    ["implicit" "explicit"] @context
    "operator" @context
    type: (_) @name
) @item

(indexer_declaration
    "this" @name
    parameters: (bracketed_parameter_list) @context
) @item

(property_declaration
    type: (identifier)? @context
    type: (predefined_type)? @context
    name: (identifier) @name
) @item

(event_declaration
    "event" @context
    name: (identifier) @name
) @item

(event_field_declaration
    "event" @context
    (variable_declaration
        (variable_declarator
            name: (identifier) @name))
) @item

(field_declaration
    (variable_declaration) @context
) @item
