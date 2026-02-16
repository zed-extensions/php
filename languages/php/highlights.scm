(php_tag) @tag
(php_end_tag) @tag

; Types

(primitive_type) @type.builtin
(cast_type) @type.builtin
(named_type (name) @type) @type
(named_type (qualified_name) @type) @type

; Named arguments (PHP 8+)

(argument
  name: (name) @variable.parameter)

; Functions

(array_creation_expression "array" @function.builtin)
(list_literal "list" @function.builtin)

(method_declaration
  name: (name) @function.method)

(function_call_expression
  function: [(qualified_name (name)) (name)] @function)

(scoped_call_expression
  name: (name) @function)

(member_call_expression
  name: (name) @function.method)

(nullsafe_member_call_expression
  name: (name) @function.method)

(function_definition
  name: (name) @function)

; Member

(property_element
  (variable_name) @property)

(member_access_expression
  name: (variable_name (name)) @property)
(member_access_expression
  name: (name) @property)
(nullsafe_member_access_expression
  name: (variable_name (name)) @property)
(nullsafe_member_access_expression
  name: (name) @property)

; Class constant access (e.g., Class::CONSTANT)

(class_constant_access_expression
  (_) (name) @constant)

; Special classes

(relative_scope) @constructor

((object_creation_expression (name) @constructor)
 (#any-of? @constructor "self" "parent"))

((binary_expression
  operator: "instanceof"
  right: (name) @constructor)
 (#any-of? @constructor "self" "parent"))

; Variables

((name) @constructor
 (#match? @constructor "^[A-Z]"))

((name) @constant
 (#match? @constant "^_?[A-Z][A-Z\\d_]+$"))
((name) @constant.builtin
 (#match? @constant.builtin "^__[A-Z][A-Z\d_]+__$"))

((name) @variable.builtin
 (#eq? @variable.builtin "this"))

(variable_name) @variable

; Basic tokens
[
  (string)
  (string_content)
  (encapsed_string)
  (heredoc)
  (heredoc_body)
  (nowdoc_body)
] @string
(boolean) @constant.builtin
(null) @constant.builtin
(integer) @number
(float) @number
(comment) @comment

; Operators

[
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "**="
  ".="
  "??="
  "&="
  "|="
  "^="
  "<<="
  ">>="

  "+"
  "-"
  "*"
  "/"
  "%"
  "**"
  "."

  "=="
  "!="
  "==="
  "!=="
  "<"
  ">"
  "<="
  ">="
  "<=>"

  "&&"
  "||"
  "!"

  "??"
  "?"
  ":"

  "&"
  "|"
  "^"
  "~"
  "<<"
  ">>"

  "++"
  "--"

  "@"

  "$"
] @operator

; punctuation

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

; "$" @punctuation.special

; Keywords

"abstract" @keyword
"and" @keyword
"as" @keyword
"break" @keyword
"case" @keyword
"catch" @keyword
"class" @keyword
"clone" @keyword
"const" @keyword
"continue" @keyword
"declare" @keyword
"default" @keyword
"do" @keyword
"echo" @keyword
"else" @keyword
"elseif" @keyword
"enum" @keyword
"enddeclare" @keyword
"endfor" @keyword
"endforeach" @keyword
"endif" @keyword
"endswitch" @keyword
"endwhile" @keyword
"extends" @keyword
"final" @keyword
"readonly" @keyword
"finally" @keyword
"for" @keyword
"foreach" @keyword
"fn" @keyword
"function" @keyword
"global" @keyword
"goto" @keyword
"if" @keyword
"implements" @keyword
"include_once" @keyword
"include" @keyword
"instanceof" @keyword
"insteadof" @keyword
"interface" @keyword
"match" @keyword
"namespace" @keyword
"new" @keyword
"or" @keyword
"print" @keyword
"private" @keyword
"protected" @keyword
"public" @keyword
"readonly" @keyword
"require_once" @keyword
"require" @keyword
"return" @keyword
"static" @keyword
"switch" @keyword
"throw" @keyword
"trait" @keyword
"try" @keyword
"use" @keyword
"while" @keyword
"xor" @keyword
"yield" @keyword
"yield from" @keyword
