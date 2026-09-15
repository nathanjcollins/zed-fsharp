; Zed override query - defines the scopes consumed by config.toml's `not_in`
; bracket guards (quote autoclose off inside strings; comments keep autoclose,
; matching Zed's Python/Rust convention - quotes in comments are usually balanced).

[
  (line_comment)
  (block_comment)
  (xml_doc_comment)
  (block_doc_comment)
] @comment

[
  (string_literal)
  (verbatim_string)
  (triple_quoted_string)
  (multidollar_string)
  (char_literal)
] @string

; Interpolated strings: scope only the text chunks, so autoclose keeps
; working in the code inside `{…}` holes.
(interpolated_string (string_content) @string)
(interpolated_verbatim_string (string_content) @string)
(interpolated_triple_string (string_content) @string)
