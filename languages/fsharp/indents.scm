; Zed indent query (Zed's @indent/@start/@end dialect — unrelated to Helix's
; ../indents.scm). Offside constructs are regex rules in the ext's config.toml.

; Bracket interiors indent one level; the closer re-aligns with the opener line.
; @start pins the baseline to the OPEN token's row, not the node's start row —
; they differ for computation_expression, whose node begins at a zero-width
; scanner token on the PREVIOUS line (`li () {}` after `=`), which would
; otherwise re-align the `}` with the `let` line.
(_ "(" @start ")" @end) @indent
(_ "[" @start "]" @end) @indent
(_ "{" @start "}" @end) @indent
(_ "[|" @start "|]" @end) @indent
(_ "{|" @start "|}" @end) @indent
(_ "[<" @start ">]" @end) @indent
(begin_end_expression "end" @end) @indent

; Block starters — the `valid_after` tokens that config.toml's
; decrease_indent_patterns re-align with (else→if, with/finally→try).
(if_expression) @start.if
(try_expression) @start.try
(match_expression) @start.match
