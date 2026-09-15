; Zed text objects (vim mode: ]m/[m + af/if, ]]/[[ + ac/ic, gc).
; Zed only reads function/class/comment captures. Deliberate differences from
; ../textobjects.scm (Helix): lambdas & `function` expressions are NOT
; functions (Zed guidance: closures don't count); local lets excluded so ]m
; stays method-level; modules count as classes; adjacent comment lines join
; into one object.

; --- Functions ---
(let_binding body: (_) @function.inside) @function.around
(let_and_binding body: (_) @function.inside) @function.around
(member_defn body: (_) @function.inside) @function.around
; abstract member Foo: int - no body, so around only (if falls back to it).
(abstract_member_defn) @function.around
(property_accessor body: (_) @function.inside) @function.around
(secondary_constructor body: (_) @function.inside) @function.around

; --- Classes: types + modules (the "large section" granularity) ---
(module_decl) @class.around
(type_decl "=" . (_) @class.inside) @class.around
(type_and_decl "=" . (_) @class.inside) @class.around
(type_extension) @class.around
(exception_decl) @class.around
(class_type_defn) @class.around
(struct_type_defn) @class.around
(interface_type_defn) @class.around

; --- Comments (`+` joins adjacent lines into one gc object) ---
(line_comment)+ @comment.around
(xml_doc_comment)+ @comment.around
(block_comment) @comment.around
(block_doc_comment) @comment.around
