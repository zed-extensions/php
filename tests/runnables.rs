//! Query-level tests for `languages/php/runnables.scm`, run with `cargo test`.
//!
//! Mirrors the approach used by `zed-extensions/java`: run the shipped `.scm`
//! over PHP source with the native tree-sitter engine and the pinned grammar,
//! under Zed's 64-match limit (see `support`). These cover the PHPUnit and Pest
//! runnables that ship on `main`.
//!
//! The `#[ignore]`d tests at the bottom document cases `main` gets wrong — it
//! tags things as PHPUnit that most likely are not. They assert the intended
//! behaviour, so they fail today and are skipped in CI; run them with
//! `cargo test -- --ignored`. These gaps are addressed by the Testo runnables
//! work on a separate branch.

mod support;

use support::{Run, run_query};

const SCM: &str = "languages/php/runnables.scm";

fn tags_for<'a>(runs: &'a [Run], text: &str) -> Vec<&'a str> {
    runs.iter()
        .filter(|r| r.text == text)
        .map(|r| r.tag.as_deref().unwrap_or("(none)"))
        .collect()
}

fn has(runs: &[Run], text: &str, tag: &str) -> bool {
    runs.iter()
        .any(|r| r.text == text && r.tag.as_deref() == Some(tag))
}

// --- PHPUnit: what `main` detects ----------------------------------------

#[test]
fn phpunit_naming_convention() {
    let src = r#"<?php
use PHPUnit\Framework\TestCase;
class CalculatorTest extends TestCase {
    public function testAdds(): void {}
    public function helper(): void {}
}
"#;
    let runs = run_query(SCM, src);

    // `test*` method -> a run icon on the method name.
    assert_eq!(tags_for(&runs, "testAdds"), vec!["phpunit-test"]);
    // `*Test` class -> a run-all icon on the class name.
    assert!(has(&runs, "CalculatorTest", "phpunit-test"));
    // A non-`test*` helper is not a test.
    assert!(runs.iter().all(|r| r.text != "helper"));
}

#[test]
fn phpunit_at_test_annotation() {
    let src = r#"<?php
use PHPUnit\Framework\TestCase;
class OrderTest extends TestCase {
    /** @test */
    public function itPays(): void {}
}
"#;
    let runs = run_query(SCM, src);
    // A method whose name is not `test*` but which carries an `@test` docblock.
    assert!(has(&runs, "itPays", "phpunit-test"));
    assert!(has(&runs, "OrderTest", "phpunit-test"));
}

#[test]
fn phpunit_test_attribute() {
    let src = r#"<?php
use PHPUnit\Framework\TestCase;
class UserTest extends TestCase {
    #[Test]
    public function itWorks(): void {}
}
"#;
    let runs = run_query(SCM, src);
    // A `#[Test]` attribute on a non-`test*` method.
    assert!(has(&runs, "itWorks", "phpunit-test"));
    assert!(has(&runs, "UserTest", "phpunit-test"));
}

#[test]
fn plain_class_has_no_runnables() {
    let src = r#"<?php
class Money {
    public function testable(): void {}
}
"#;
    assert!(run_query(SCM, src).is_empty());
}

// --- Pest -----------------------------------------------------------------

#[test]
fn pest_helpers_capture_the_description() {
    let src = r#"<?php
it('adds numbers', function () {});
test('subtracts numbers', function () {});
describe('math', function () {});
"#;
    let runs = run_query(SCM, src);
    assert!(has(&runs, "adds numbers", "pest-test"));
    assert!(has(&runs, "subtracts numbers", "pest-test"));
    assert!(has(&runs, "math", "pest-test"));
}

/// An abstract `*Test` class is tagged just like a concrete one. The PHPUnit
/// patterns carry a `(#not-any-eq? @_modifier "abstract")` guard and a comment
/// claiming abstract classes are excluded, but with the pinned grammar that
/// guard never fires, so the run icons appear regardless. This pins the actual
/// behaviour (PHPUnit won't execute an abstract class, but the gutter is
/// harmless — it simply finds no cases).
#[test]
fn abstract_test_class_is_tagged() {
    let src = r#"<?php
abstract class BaseTest extends TestCase {
    public function testThing(): void {}
}
"#;
    let runs = run_query(SCM, src);
    assert!(has(&runs, "BaseTest", "phpunit-test"));
    assert!(has(&runs, "testThing", "phpunit-test"));
}

// --- Known over-tagging: cases `main` gets wrong ---------------------------
//
// These assert the *intended* behaviour, so they fail against today's query
// and are marked `#[ignore]`. The Testo runnables work (separate branch) fixes
// them; here they simply document the gaps.

/// A `*Test` class with no `extends` is almost certainly not PHPUnit (PHPUnit
/// test cases always inherit from `TestCase`, directly or transitively) — it is
/// far more likely a Testo test. Today the naming-convention patterns tag it
/// (both the method and the class) as `phpunit-test` anyway.
///
/// Fixed by requiring a `base_clause` on the PHPUnit patterns.
#[test]
#[ignore = "main over-tags a *Test class with no inheritance as PHPUnit"]
fn non_inheriting_test_class_should_not_be_phpunit() {
    let src = r#"<?php
class SyncDepsTest {
    public function testRuns(): void {}
}
"#;
    let runs = run_query(SCM, src);
    assert!(
        !has(&runs, "testRuns", "phpunit-test"),
        "a test method in a class with no base class should not be PHPUnit"
    );
    assert!(
        !has(&runs, "SyncDepsTest", "phpunit-test"),
        "a *Test class with no base class should not be PHPUnit"
    );
}

/// A bare `#[Test]` attribute is ambiguous between PHPUnit
/// (`PHPUnit\Framework\Attributes\Test`) and Testo (`Testo\Test`). In a class
/// with no inheritance it is not PHPUnit — but `main` still tags it, purely
/// because the class name ends in `Test`.
///
/// Fixed by gating the `#[Test]` handling on inheritance / import.
#[test]
#[ignore = "main tags a bare #[Test] method in a non-inheriting class as PHPUnit"]
fn bare_test_attribute_without_inheritance_should_not_be_phpunit() {
    let src = r#"<?php
class OrderTest {
    #[Test]
    public function pays(): void {}
}
"#;
    let runs = run_query(SCM, src);
    assert!(
        !has(&runs, "pays", "phpunit-test"),
        "a bare #[Test] method in a class with no base class should not be PHPUnit"
    );
}
