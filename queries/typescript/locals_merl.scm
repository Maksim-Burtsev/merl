(variable_declarator
  name: (object_pattern
    (rest_pattern
      (identifier) @local.definition.var)))

(variable_declarator
  name: (array_pattern
    (identifier) @local.definition.var))

(for_in_statement
  left: (identifier) @local.definition.var)

(catch_clause
  parameter: (identifier) @local.definition.var)

(type_alias_declaration
  name: (type_identifier) @local.definition.type)

(interface_declaration
  name: (type_identifier) @local.definition.type)

(class_declaration
  name: (type_identifier) @local.definition.type)

(enum_declaration
  name: (identifier) @local.definition.type)
