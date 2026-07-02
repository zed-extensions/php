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

; ---------------------------------------------------------------------------
; Testo (https://php-testo.github.io) runnables.
;
; Testo detects tests by the `#[Test]` attribute rather than by naming
; convention:
;   * a class annotated with a class-level `#[Test]` — every public method
;     whose return type is `void`/`never` is a test case (other return types
;     are treated as data providers and skipped);
;   * any free function annotated with `#[Test]`.
;
; The class-level `#[Test]` is what distinguishes Testo from PHPUnit, where
; `#[Test]` (PHPUnit\Framework\Attributes\Test) is only ever placed on
; methods. A bare method-level `#[Test]` is therefore ambiguous between the
; two frameworks — tree-sitter can't tell `use Testo\Test` from
; `use PHPUnit\Framework\Attributes\Test` — so we deliberately do NOT emit a
; Testo runnable for it (that method-only Testo style is rare, and matching it
; would put Testo buttons on every PHPUnit test). A class-level or
; function-level `#[Test]` is unambiguous and is matched below.
;
; Note: abstract classes are not excluded here (tree-sitter queries can't
; assert the absence of a modifier). Testo ignores them at run time, so at
; worst a button on an abstract class runs and finds no cases.
; ---------------------------------------------------------------------------
; Public `void`/`never` method inside a class annotated with class-level #[Test]
((class_declaration
  attributes: (attribute_list
    (attribute_group
      (attribute
        [
          (name)
          (qualified_name)
        ] @_class_attr)))
  (#any-of? @_class_attr "Test" "\\Testo\\Test")
  body: (declaration_list
    (method_declaration
      (visibility_modifier) @_visibility
      (#eq? @_visibility "public")
      name: (_) @run
      return_type: (_) @_rtype
      (#any-of? @_rtype "void" "never")))) @_testo-test
  (#set! tag testo-test))

; Class annotated with a class-level #[Test] attribute (run the whole case)
((class_declaration
  attributes: (attribute_list
    (attribute_group
      (attribute
        [
          (name)
          (qualified_name)
        ] @_class_attr)))
  (#any-of? @_class_attr "Test" "\\Testo\\Test")
  name: (_) @run) @_testo-test
  (#set! tag testo-test))

; Free function annotated with a #[Test] attribute
((function_definition
  attributes: (attribute_list
    (attribute_group
      (attribute
        [
          (name)
          (qualified_name)
        ] @_fn_attr)))
  (#any-of? @_fn_attr "Test" "\\Testo\\Test")
  name: (_) @run) @_testo-test
  (#set! tag testo-test))

; Method-level #[Test] disambiguated to Testo via the file's `use` import.
; PHP forbids importing two different classes under the same alias, so once a
; file contains `use Testo\Test;` every unqualified `#[Test]` in it is Testo's
; — that is how we tell a method-level Testo test apart from a PHPUnit one
; (`PHPUnit\Framework\Attributes\Test`) without semantic resolution. The `use`
; and the class have to be matched through a shared ancestor.
;
; Form 1: no namespace, or the `namespace X;` (semicolon) form — the `use` and
; the class are siblings under the program root.
((program
  (namespace_use_declaration
    (namespace_use_clause
      (qualified_name) @_use))
  (#match? @_use "^Testo\\\\Test$")
  (class_declaration
    body: (declaration_list
      (method_declaration
        attributes: (attribute_list
          (attribute_group
            (attribute
              (name) @_attr)))
        (#eq? @_attr "Test")
        name: (_) @run)))) @_testo-test
  (#set! tag testo-test))

; Form 2: the braced `namespace X { ... }` form — the `use` and the class live
; inside the namespace body instead of at the program root.
((namespace_definition
  body: (compound_statement
    (namespace_use_declaration
      (namespace_use_clause
        (qualified_name) @_use))
    (#match? @_use "^Testo\\\\Test$")
    (class_declaration
      body: (declaration_list
        (method_declaration
          attributes: (attribute_list
            (attribute_group
              (attribute
                (name) @_attr)))
          (#eq? @_attr "Test")
          name: (_) @run))))) @_testo-test
  (#set! tag testo-test))

; Method carrying a fully-qualified `#[\Testo\Test]` attribute. A fully
; qualified name is self-identifying, so no `use` correlation is needed and it
; is unambiguous regardless of the class name.
((method_declaration
  attributes: (attribute_list
    (attribute_group
      (attribute
        (qualified_name) @_attr)))
  (#eq? @_attr "\\Testo\\Test")
  name: (_) @run) @_testo-test
  (#set! tag testo-test))

; Testo configuration file: `return new ApplicationConfig(...)`. Runs the whole
; suite defined by this config via `testo --config=<file>`.
((return_statement
  (object_creation_expression
    [
      (name)
      (qualified_name)
    ] @run
    (#match? @run "(^|\\\\)ApplicationConfig$"))) @_testo-config
  (#set! tag testo-config))

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
