(namespace_declaration
    "namespace" @context
    name: (qualified_name) @name
) @item

(file_scoped_namespace_declaration
    "namespace" @context
    name: (qualified_name) @name
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

(constructor_declaration
    name: (identifier) @name
) @item

(method_declaration
    name: (identifier) @name
    parameters: (parameter_list) @context
) @item

(property_declaration
    type: (identifier)? @context
    type: (predefined_type)? @context
    name: (identifier) @name
) @item

(field_declaration
    (variable_declaration) @context
) @item
