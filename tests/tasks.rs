//! Consistency tests for `languages/php/tasks.json`, run with `cargo test`.
//! Mirrors `zed-extensions/java`'s `task_verification_test.rs`: a runnable tag
//! is useless without a task that carries it, so check the two files agree.

use serde_json::Value;
use std::collections::HashSet;

const TASKS: &str = "languages/php/tasks.json";
const SCM: &str = "languages/php/runnables.scm";

fn tasks() -> Vec<Value> {
    let json = std::fs::read_to_string(TASKS).expect("read tasks.json");
    let parsed: Value = serde_json::from_str(&json).expect("parse tasks.json");
    parsed.as_array().expect("tasks.json is an array").clone()
}

/// The `command` of the first task carrying `tag`.
fn command_for_tag(tag: &str) -> String {
    for t in tasks() {
        let carries = t["tags"]
            .as_array()
            .map(|a| a.iter().any(|x| x.as_str() == Some(tag)))
            .unwrap_or(false);
        if carries {
            return t["command"].as_str().unwrap().to_string();
        }
    }
    panic!("no task carries tag `{tag}`");
}

#[test]
fn runnable_tags_map_to_the_expected_runner() {
    assert_eq!(command_for_tag("phpunit-test"), "./vendor/bin/phpunit");
    assert_eq!(command_for_tag("pest-test"), "./vendor/bin/pest");
}

/// Every tag emitted by a `(#set! tag <tag>)` in runnables.scm must have at
/// least one task in tasks.json, otherwise the gutter icon would do nothing.
#[test]
fn every_runnable_tag_has_a_task() {
    let scm = std::fs::read_to_string(SCM).expect("read runnables.scm");
    let task_tags: HashSet<String> = tasks()
        .iter()
        .filter_map(|t| t["tags"].as_array())
        .flatten()
        .filter_map(|v| v.as_str())
        .map(String::from)
        .collect();

    for line in scm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("(#set! tag ") {
            let tag = rest.trim_end_matches(')').trim();
            assert!(
                task_tags.contains(tag),
                "runnable tag `{tag}` has no task in tasks.json"
            );
        }
    }
}
