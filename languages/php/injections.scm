((text) @injection.content
 (#set! injection.language "html")
 (#set! injection.combined))

((string_value) @injection.content
 (#match? @injection.content "(?i)^\\s*(select|update|insert|delete|show|begin|use|drop|truncate|create|alter)")
 (#set! injection.language "sql"))

((comment) @injection.content
  (#match? @injection.content "^/\\*\\*[^*]")
  (#set! injection.language "phpdoc"))

((heredoc_body) (heredoc_end) @injection.language) @injection.content

((nowdoc_body) (heredoc_end) @injection.language) @injection.content
