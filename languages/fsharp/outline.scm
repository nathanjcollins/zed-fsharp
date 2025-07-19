; Namespaces
((namespace
  name: (long_identifier) @name) @item
 (#set! "kind" "namespace"))

; Named modules
((named_module
  name: (long_identifier) @name) @item
 (#set! "kind" "module"))

; Module definitions  
((module_defn
  (identifier) @name) @item
 (#set! "kind" "module"))

; Type definitions - Record types
((type_definition
  (record_type_defn
    (type_name (identifier) @name))) @item
 (#set! "kind" "struct"))

; Type definitions - Union types
((type_definition
  (union_type_defn
    (type_name (identifier) @name))) @item
 (#set! "kind" "enum"))

; Type definitions - Enum types
((type_definition
  (enum_type_defn
    (type_name (identifier) @name))) @item
 (#set! "kind" "enum"))

; Type definitions - Anonymous types (classes/interfaces)
((type_definition
  (anon_type_defn
    (type_name (identifier) @name))) @item
 (#set! "kind" "class"))

; Type definitions - Type abbreviations
((type_definition
  (type_abbrev_defn
    (type_name (identifier) @name))) @item
 (#set! "kind" "type"))

; Type definitions - Delegate types
((type_definition
  (delegate_type_defn
    (type_name (identifier) @name))) @item
 (#set! "kind" "interface"))

; Function definitions (let bindings)
((value_declaration
  (function_or_value_defn
    (function_declaration_left
      (identifier) @name))) @item
 (#set! "kind" "function"))

; Value definitions (let bindings)
((value_declaration
  (function_or_value_defn
    (value_declaration_left
      (identifier_pattern
        (long_identifier
          (identifier) @name))))) @item
 (#set! "kind" "variable"))

; Member definitions - methods and properties
((member_defn
  (member_signature
    (identifier) @name)) @item
 (#set! "kind" "method"))

; Union type cases
((union_type_case
  (identifier) @name) @item
 (#set! "kind" "variant"))

; Enum type cases
((enum_type_case
  (identifier) @name) @item
 (#set! "kind" "variant"))