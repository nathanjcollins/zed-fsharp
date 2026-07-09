; Zed indent query (Zed's @indent/@start/@end dialect — unrelated to Helix's
; ../indents.scm). Offside constructs are regex rules in the ext's config.toml.

; Bracket interiors indent one level; the closer re-aligns with the opener line.
(_ "(" ")" @end) @indent
(_ "[" "]" @end) @indent
(_ "{" "}" @end) @indent
(_ "[|" "|]" @end) @indent
(_ "{|" "|}" @end) @indent
(_ "[<" ">]" @end) @indent
(begin_end_expression "end" @end) @indent

; Block starters — the `valid_after` tokens that config.toml's
; decrease_indent_patterns re-align with (else→if, with/finally→try).
(if_expression) @start.if
(try_expression) @start.try
(match_expression) @start.match
