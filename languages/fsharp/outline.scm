; Zed outline query - powers the outline panel, breadcrumbs, and in-file
; symbol search. `let_binding` only matches declaration-position lets
; (module/class level); local lets are `let_decl_indented`, deliberately
; excluded to keep the outline at API altitude.

(xml_doc_comment) @annotation
(block_doc_comment) @annotation

(namespace_decl
  "namespace" @context
  name: (long_identifier) @name) @item

(module_decl
  "module" @context
  name: (long_identifier) @name) @item

(type_decl
  "type" @context
  name: (identifier) @name) @item

(type_and_decl
  "and" @context
  name: (identifier) @name) @item

(exception_decl
  "exception" @context
  name: (identifier) @name) @item

(union_case
  name: (identifier) @name) @item

(enum_case
  name: (identifier) @name) @item

(record_type_field
  name: (identifier) @name) @item

(let_binding
  "let" @context
  "rec"? @context
  name: [(identifier) (operator_name) (active_pattern_name)] @name) @item

(let_and_binding
  "and" @context
  name: (identifier) @name) @item

(member_defn
  "static"? @context
  ["member" "override" "default"] @context
  name: (identifier) @name) @item

(abstract_member_defn
  "abstract" @context
  name: (identifier) @name) @item

(val_field
  "val" @context
  name: (identifier) @name) @item
