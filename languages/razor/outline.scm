; Razor-specific outline items

(razor_section
  "at_section" @context
  (identifier) @name
) @item

; Inherited from C# (components may define these inline)
(class_declaration
  "class" @context
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
