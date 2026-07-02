; Class that follow the naming convention of PHPUnit test classes
; and that doesn't have the abstract modifier
; and extends a base class (PHPUnit test classes always inherit from TestCase,
; directly or transitively; a *Test class with no `extends` at all is not
; PHPUnit — most likely a Testo test — so requiring a base_clause avoids
; tagging those as phpunit-test)
; and have a method that follow the naming convention of PHPUnit test methods
; and the method is public
((class_declaration
  (_)* @_modifier
  (#not-any-eq? @_modifier "abstract")
  .
  name: (_) @_name
  (#match? @_name ".*Test$")
  (base_clause)
  body: (declaration_list
    (method_declaration
      (visibility_modifier)? @_visibility
      (#eq? @_visibility "public")
      name: (_) @run
      (#match? @run "^test.*")))) @_phpunit-test
  (#set! tag phpunit-test))

; Class that follow the naming convention of PHPUnit test classes
; and that doesn't have the abstract modifier
; and extends a base class (see note above — filters out inheritance-less
; Testo classes)
; and have a method that has the @test annotation
; and the method is public
((class_declaration
  (_)* @_modifier
  (#not-any-eq? @_modifier "abstract")
  .
  name: (_) @_name
  (#match? @_name ".*Test$")
  (base_clause)
  body: (declaration_list
    ((comment) @_comment
      (#match? @_comment ".*@test\\b.*")
      .
      (method_declaration
        (visibility_modifier)? @_visibility
        (#eq? @_visibility "public")
        name: (_) @run
        (#not-match? @run "^test.*"))))) @_phpunit-test
  (#set! tag phpunit-test))

; Method carrying the #[Test] attribute, disambiguated to PHPUnit via the
; file's `use` import. Both PHPUnit (PHPUnit\Framework\Attributes\Test) and
; Testo (Testo\Test) expose a method-level #[Test]; since PHP forbids two
; imports sharing an alias, the presence of `use PHPUnit\Framework\Attributes\Test`
; proves the attribute is PHPUnit's. This replaces the old class-name (*Test)
; gate so that a Testo file's #[Test] methods are no longer tagged phpunit-test.
; (Trade-off: a PHPUnit file importing the attribute via a group use or writing
; it fully-qualified won't match here — it still gets class-level buttons.)
;
; Form 1: no namespace, or `namespace X;` — `use` and class are siblings.
((program
  (namespace_use_declaration
    (namespace_use_clause
      (qualified_name) @_use))
  (#eq? @_use "PHPUnit\\Framework\\Attributes\\Test")
  (class_declaration
    body: (declaration_list
      (method_declaration
        attributes: (attribute_list
          (attribute_group
            (attribute
              (name) @_attribute)))
        (#eq? @_attribute "Test")
        (visibility_modifier)? @_visibility
        (#eq? @_visibility "public")
        name: (_) @run
        (#not-match? @run "^test.*"))))) @_phpunit-test
  (#set! tag phpunit-test))

; Form 2: braced `namespace X { ... }`.
((namespace_definition
  body: (compound_statement
    (namespace_use_declaration
      (namespace_use_clause
        (qualified_name) @_use))
    (#eq? @_use "PHPUnit\\Framework\\Attributes\\Test")
    (class_declaration
      body: (declaration_list
        (method_declaration
          attributes: (attribute_list
            (attribute_group
              (attribute
                (name) @_attribute)))
          (#eq? @_attribute "Test")
          (visibility_modifier)? @_visibility
          (#eq? @_visibility "public")
          name: (_) @run
          (#not-match? @run "^test.*")))))) @_phpunit-test
  (#set! tag phpunit-test))

; Class that follow the naming convention of PHPUnit test classes
; and that doesn't have the abstract modifier
; and extends a base class (see note above — filters out inheritance-less
; Testo classes)
((class_declaration
  (_)* @_modifier
  (#not-any-eq? @_modifier "abstract")
  .
  name: (_) @run
  (#match? @run ".*Test$")
  (base_clause)) @_phpunit-test
  (#set! tag phpunit-test))

; Method carrying a fully-qualified `#[\PHPUnit\Framework\Attributes\Test]`
; attribute — self-identifying, so no `use` correlation is needed.
((method_declaration
  attributes: (attribute_list
    (attribute_group
      (attribute
        (qualified_name) @_attribute)))
  (#eq? @_attribute "\\PHPUnit\\Framework\\Attributes\\Test")
  (visibility_modifier)? @_visibility
  (#eq? @_visibility "public")
  name: (_) @run) @_phpunit-test
  (#set! tag phpunit-test))

; Add support for Pest runnable
; Function expression that has `it`, `test` or `describe` as the function name
((function_call_expression
  function: (_) @_name
  (#any-of? @_name "it" "test" "describe")
  arguments: (arguments
    .
    (argument
      [
        (encapsed_string
          (string_content) @run)
        (string
          (string_content) @run)
      ]))) @_pest-test
  (#set! tag pest-test))
