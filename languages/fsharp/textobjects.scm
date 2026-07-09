; F# text-object queries for Helix.
;
; Helix surfaces these as `mif`/`maf` (function), `mit`/`mat` (type),
; `mia`/`maa` (argument), `mic`/`mac` (comment) — `mi…` for "inside",
; `ma…` for "around".
;
; `.inside` captures use the `body:` field where the grammar exposes it.
; Rules without a body field (`type_decl`, `type_and_decl`) fall back to
; positional anchoring (`"=" . (_)`).

; ── Functions ────────────────────────────────────────────────────────────────
; let foo x = body
(let_binding body: (_) @function.inside) @function.around

; Nested `let foo = body` inside an enclosing expression
(let_decl_indented body: (_) @function.inside) @function.around

; `and foo = body` (mutual-recursion continuation)
(let_and_binding body: (_) @function.inside) @function.around

; member this.Foo x = body  /  static member Foo x = body  /
; member val Auto = expr [with get [, set]]
(member_defn body: (_) @function.inside) @function.around

; abstract member Foo: int  — no body, so no .inside
(abstract_member_defn) @function.around

; with get () = body  /  with set v = body
(property_accessor body: (_) @function.inside) @function.around

; new (args) = body [then expr]
(secondary_constructor body: (_) @function.inside) @function.around

; fun x -> body
(lambda_expression body: (_) @function.inside) @function.around

; function | pat -> expr | pat -> expr  — body is a list of arms, capture as a whole
(function_expression) @function.around

; ── Types / classes ──────────────────────────────────────────────────────────
; `type Foo = record/union/etc.` — body is whatever follows `=`.
(type_decl
  "=" . (_) @class.inside) @class.around

; Same shape for `and Foo = …`
(type_and_decl
  "=" . (_) @class.inside) @class.around

; `type Foo with` — member impls follow as siblings; only @around is meaningful.
(type_extension) @class.around

; exception Foo of …
(exception_decl) @class.around

; Explicit block bodies — selecting the block as its own @class is useful when
; you want to grab just the class/struct/interface body in `type Foo = class … end`.
(class_type_defn) @class.around
(struct_type_defn) @class.around
(interface_type_defn) @class.around

; ── Arguments / parameters ───────────────────────────────────────────────────
; Curried parameters (`x`, `(x: int)`, destructuring forms…) and OOP-style
; tuple parameters (`x: int` inside `(x: int, y: int)`).
(parameter) @parameter.inside @parameter.around
(tuple_param) @parameter.inside @parameter.around

; ── Comments ─────────────────────────────────────────────────────────────────
[
  (line_comment)
  (xml_doc_comment)
  (block_comment)
  (block_doc_comment)
] @comment.inside @comment.around

; Note: there is no query-level way to make `maf` / `mat` extend over an
; adjacent doc comment or attribute from inside the function body. Captures
; bound ranges, and tree-sitter queries can't union them across siblings.
; The fix is grammar-level (make the comment/attribute a child of the decl).
; See LIMITATIONS.md.
