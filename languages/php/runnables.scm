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

; NOTE: a short method-level `#[Test]` is ambiguous between PHPUnit
; (PHPUnit\Framework\Attributes\Test) and Testo (Testo\Test). The only precise
; disambiguator is the file's `use` import, but correlating it with the method
; requires a query rooted at `program`/`namespace` that spans the whole file —
; such patterns create one in-progress match state per (use-statement × method)
; pair, which blows past tree-sitter's match limit on real files and silently
; drops later runnables (gutters vanish from some line downward). So bare
; `#[Test]` is handled locally by the Testo section instead; PHPUnit here relies
; on its naming convention (`*Test` class + `test*`/`@test`) and on the
; fully-qualified `#[\PHPUnit\Framework\Attributes\Test]` below.
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
; A bare method-level `#[Test]` is ambiguous between Testo and PHPUnit, and the
; only exact disambiguator (the file's `use` import) can only be correlated by a
; `program`-rooted query that blows past tree-sitter's match limit on real files
; (dropping later runnables). We therefore match `#[Test]` LOCALLY and treat it
; as Testo; PHPUnit keeps its naming-convention / fully-qualified detection.
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

; Class whose #[Test] lives on the methods (not on the class) still gets a
; run-all gutter on the class name, so the whole class can be run at once.
; Rooted at the class (never at `program`), so it stays linear; it fires once
; per matching method, but those all land on the class-name row and Zed collapses
; them into a single indicator.
((class_declaration
  name: (_) @run
  body: (declaration_list
    (method_declaration
      attributes: (attribute_list
        (attribute_group
          (attribute
            [
              (name)
              (qualified_name)
            ] @_attr)))
      (#any-of? @_attr "Test" "\\Testo\\Test")))) @_testo-test
  (#set! tag testo-test))

; Run-all icon on the name. Matched LOCALLY (rooted at the declaration), never at
; `program`, so there is no per-(use-statement × method) match-state blow-up.
;
; Unambiguously Testo: a fully-qualified `#[\Testo\Test]` (method or function),
; and a bare `#[Test]` on a free function (PHPUnit has no function tests).
([
  (method_declaration
    attributes: (attribute_list
      (attribute_group
        (attribute
          (qualified_name) @_attr)))
    name: (_) @run)
  (function_definition
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @_attr)))
    name: (_) @run)
] @_testo-test
  (#any-of? @_attr "Test" "\\Testo\\Test")
  (#set! tag testo-test))

; A bare method-level `#[Test]` is ambiguous between Testo and PHPUnit (the only
; exact disambiguator is the file's `use` import, which can't be correlated
; without the `program`-rooted pattern that made gutters vanish). Instead of
; guessing, the ambiguity is split by position: the Testo run sits on the
; attribute row (the `testo-type-test` icon above, `--type=test`), and the
; method-name row gets the PHPUnit run — so the user picks by where they click.
;
; The PHPUnit run is only offered when the class extends something: PHPUnit tests
; always extend `TestCase`, so a bare `#[Test]` in a class with no `extends` is
; not PHPUnit and its method-name row stays empty. Rooted at the class (like the
; naming-convention PHPUnit patterns above), so it stays linear — the `base_clause`
; is a single node, not a per-`use` multiplier.
((class_declaration
  (base_clause)
  body: (declaration_list
    (method_declaration
      attributes: (attribute_list
        (attribute_group
          (attribute
            (name) @_attr)))
      (#eq? @_attr "Test")
      name: (_) @run))) @_phpunit-test
  (#set! tag phpunit-test))

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

; ---------------------------------------------------------------------------
; Testo — typed attribute runnables (methods and free functions).
;
; Each test-kind attribute gets its own gutter icon anchored on the attribute
; itself, running only that kind via `--type=<kind>`. The icon sits on the
; attribute's row because `@run` is placed on the attribute node; `$ZED_SYMBOL`
; still resolves to the enclosing method/function (its outline item spans the
; attribute lines), so `--filter` stays symbol-scoped. Each kind matches both a
; `method_declaration` and a `function_definition` via a `[...]` alternation.
;
; The kinds have Testo-unique names and are matched by bare name or FQN
; directly, locally (never rooted at `program`, to avoid the match-state
; blow-up described in the PHPUnit section). `#[Test]` is treated as Testo.
; ---------------------------------------------------------------------------
; #[Test] / #[\Testo\Test] on a method or free function -> --type=test.
([
  (method_declaration
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @run))))
  (function_definition
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @run))))
] @_testo-type-test
  (#any-of? @run "Test" "\\Testo\\Test")
  (#set! tag testo-type-test))

; #[TestInline] (\Testo\Inline\TestInline) -> --type=inline. Repeatable, so a
; symbol may carry several; each occurrence is a separate match and thus its own
; icon. The 0-based ordinal that Testo accepts as `--filter=<symbol>:<n>` cannot
; be derived by tree-sitter (it can't count filtered siblings, and Zed exposes
; no such variable), so the filter stays symbol-level and every inline case is
; run.
([
  (method_declaration
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @run))))
  (function_definition
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @run))))
] @_testo-type-inline
  (#any-of? @run "TestInline" "\\Testo\\Inline\\TestInline")
  (#set! tag testo-type-inline))

; #[Bench] (\Testo\Bench) -> --type=bench.
([
  (method_declaration
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @run))))
  (function_definition
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @run))))
] @_testo-type-bench
  (#any-of? @run "Bench" "\\Testo\\Bench")
  (#set! tag testo-type-bench))

; #[TestRectorFixtures(...)] (\Testo\Bridge\Rector\Testing\TestRectorFixtures) is
; a TARGET_CLASS attribute marking a Rector rule whose `*.php.inc` fixtures are the
; test cases -> --type=rector-fixture. Unlike the kinds above it sits on the class,
; so this matches a `class_declaration`; `$ZED_SYMBOL` resolves to the class name
; (the class outline item spans its attribute lines). The attribute's argument list
; (the fixture path) doesn't affect the match — `(name)` is still its first child.
((class_declaration
  attributes: (attribute_list
    (attribute_group
      (attribute
        [
          (name)
          (qualified_name)
        ] @run)))
  (#any-of? @run "TestRectorFixtures" "\\Testo\\Bridge\\Rector\\Testing\\TestRectorFixtures")) @_testo-type-rector-fixture
  (#set! tag testo-type-rector-fixture))

; Run-all icon on the symbol name for the non-#[Test] kinds (method or free
; function): clicking it runs every case of the symbol, no `--type`. #[Test]
; symbols already get a run-all icon from the generic patterns above; this adds
; the same for symbols whose only marker is #[TestInline] / #[Bench] /
; #[RectorTestingPlugin]. It fires once per matching attribute, so a symbol with
; several (repeated or mixed kinds) yields duplicate matches on the same row —
; Zed keys runnables by row, collapsing them into a single gutter indicator.
([
  (method_declaration
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @_attr)))
    name: (_) @run)
  (function_definition
    attributes: (attribute_list
      (attribute_group
        (attribute
          [
            (name)
            (qualified_name)
          ] @_attr)))
    name: (_) @run)
] @_testo-test
  (#any-of? @_attr "TestInline" "\\Testo\\Inline\\TestInline" "Bench" "\\Testo\\Bench")
  (#set! tag testo-test))

; Run-all icon on the class name for a #[TestRectorFixtures] rule class (the
; class-level counterpart of the run-all pattern above).
((class_declaration
  attributes: (attribute_list
    (attribute_group
      (attribute
        [
          (name)
          (qualified_name)
        ] @_attr)))
  (#any-of? @_attr "TestRectorFixtures" "\\Testo\\Bridge\\Rector\\Testing\\TestRectorFixtures")
  name: (_) @run) @_testo-test
  (#set! tag testo-test))

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
