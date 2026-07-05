((text) @injection.content
  (#set! injection.language "html")
  (#set! injection.combined))

((comment) @injection.content
  (#match? @injection.content "^/\\*\\*[^*]")
  (#set! injection.language "phpdoc"))

((comment) @injection.content
  (#set! injection.language "comment"))

; Inject the heredoc/nowdoc body as the language named by its tag. Zed resolves
; the tag against language names and file extensions, case-insensitively, so
; `<<<SQL`, `<<<HTML`, `<<<JSON`, `<<<JS`, `<<<YML`, `<<<MD`, ... all work.
((heredoc_body)
  (heredoc_end) @injection.language) @injection.content

((nowdoc_body)
  (heredoc_end) @injection.language) @injection.content

; Highlight the pattern argument of the PCRE functions as a regular expression.
; Only functions whose first argument is a pattern (not `preg_quote`, whose
; first argument is a literal string to escape).
(function_call_expression
  function: (name) @_preg
  arguments: (arguments
    .
    (argument
      [
        (string
          (string_content) @injection.content)
        (encapsed_string
          (string_content) @injection.content)
      ]))
  (#any-of? @_preg
    "preg_match" "preg_match_all" "preg_replace" "preg_replace_callback" "preg_split" "preg_grep")
  (#set! injection.language "regex"))

; Highlight printf-style format strings (format is the first argument).
(function_call_expression
  function: (name) @_fmt
  arguments: (arguments
    .
    (argument
      [
        (string
          (string_content) @injection.content)
        (encapsed_string
          (string_content) @injection.content)
      ]))
  (#any-of? @_fmt "sprintf" "printf" "vsprintf" "vprintf")
  (#set! injection.language "printf"))

; ... and where the format is the second argument.
(function_call_expression
  function: (name) @_fmt
  arguments: (arguments
    .
    (argument)
    .
    (argument
      [
        (string
          (string_content) @injection.content)
        (encapsed_string
          (string_content) @injection.content)
      ]))
  (#any-of? @_fmt "fprintf" "vfprintf" "sscanf" "fscanf")
  (#set! injection.language "printf"))
