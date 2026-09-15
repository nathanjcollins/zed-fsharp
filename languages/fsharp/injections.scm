; Zed injection query - mirrors ../injections.scm (Helix); Zed accepts the
; same standard captures/properties. Keep the shared rules in sync.

; (** … *) doc comments hold markdown.
((block_doc_comment) @injection.content
 (#set! injection.language "markdown"))

; /// doc lines combined into one XML document (closing tags span lines).
; No-op unless Zed's XML extension is installed.
((xml_doc_comment) @injection.content
 (#set! injection.language "xml")
 (#set! injection.combined))

; TODO:/FIXME: markers via the community "comment" extension (same injection
; Zed's built-in Rust queries ship). No-op when it isn't installed.
([
  (line_comment)
  (block_comment)
] @injection.content
 (#set! injection.language "comment"))
