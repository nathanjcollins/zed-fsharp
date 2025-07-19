; Namespaces
(namespace
  name: (long_identifier) @name) @item
  (#set! "kind" "namespace")

; Named modules
(named_module
  name: (long_identifier) @name) @item
  (#set! "kind" "module")

; Module definitions
(module_defn
  name: (long_identifier) @name) @item
  (#set! "kind" "module")

; Record types
(record_type_defn
  type_name: (type_name) @name) @item
  (#set! "kind" "struct")

; Union types
(union_type_defn
  type_name: (type_name) @name) @item
  (#set! "kind" "enum")

; Enum types
(enum_type_defn
  type_name: (type_name) @name) @item
  (#set! "kind" "enum")

; Anonymous types (classes/interfaces)
(anon_type_defn
  type_name: (type_name) @name) @item
  (#set! "kind" "class")

; Type abbreviations
(type_abbrev_defn
  type_name: (type_name) @name) @item
  (#set! "kind" "type")

; Delegate types
(delegate_type_defn
  type_name: (type_name) @name) @item
  (#set! "kind" "interface")

; Function definitions (let bindings)
(function_or_value_defn
  function_declaration_left: (function_declaration_left
    (identifier) @name)) @item
  (#set! "kind" "function")

; Value definitions (let bindings)
(function_or_value_defn
  value_declaration_left: (value_declaration_left
    (identifier_pattern
      (long_identifier
        (identifier) @name)))) @item
  (#set! "kind" "variable")

; Class methods
(member_defn
  method_or_prop_defn: (method_or_prop_defn
    name: (property_or_ident
      (identifier) @name))) @item
  (#set! "kind" "method")

; Class properties
(member_defn
  property_declaration: (property_declaration
    (property_or_ident
      (identifier) @name))) @item
  (#set! "kind" "property")

; Abstract members
(member_defn
  abstract_member: (abstract_member
    name: (property_or_ident
      (identifier) @name))) @item
  (#set! "kind" "method")

; Interface implementations
(member_defn
  interface_implementation: (interface_implementation
    (type_name) @name)) @item
  (#set! "kind" "interface")

; Union type cases
(union_type_case
  (identifier) @name) @item
  (#set! "kind" "variant")

; Enum type cases
(enum_type_case
  (identifier) @name) @item
  (#set! "kind" "variant")

; Record fields
(record_field
  (identifier) @name) @item
  (#set! "kind" "field")