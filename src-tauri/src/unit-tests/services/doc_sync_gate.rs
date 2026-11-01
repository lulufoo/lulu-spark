use std::panic::{catch_unwind, AssertUnwindSafe};

use super::*;

#[test]
fn a_free_gate_lets_one_run_in() {
    let gate = SyncGate::new();

    assert!(gate.try_begin().is_some());
}

#[test]
fn a_second_run_is_kept_out_while_the_first_goes_on() {
    let gate = SyncGate::new();

    let first = gate.try_begin().expect("first run");

    assert!(gate.try_begin().is_none(), "the gate is held");
    assert!(gate.try_begin().is_none(), "and stays held");
    drop(first);
}

#[test]
fn the_gate_opens_when_the_run_ends() {
    let gate = SyncGate::new();
    drop(gate.try_begin().expect("first run"));

    assert!(gate.try_begin().is_some());
}

#[test]
fn clones_are_one_gate() {
    let gate = SyncGate::new();
    let other = gate.clone();

    let held = gate.try_begin().expect("first run");
    assert!(other.try_begin().is_none(), "a clone sees the run");
    drop(held);

    assert!(other.try_begin().is_some());
}

#[test]
fn separate_gates_do_not_hold_each_other() {
    let (a, b) = (SyncGate::new(), SyncGate::new());

    let _held = a.try_begin().expect("run on a");

    assert!(b.try_begin().is_some());
}

#[test]
fn a_run_that_panics_opens_the_gate() {
    let gate = SyncGate::new();

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _run = gate.try_begin().expect("run");
        panic!("the sync went wrong");
    }));

    assert!(outcome.is_err());
    assert!(gate.try_begin().is_some(), "the gate is not stuck shut");
}

#[test]
fn a_cancel_reaches_the_run_that_holds_the_gate() {
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("run");
    assert!(!run.is_cancelled(), "a run starts uncancelled");

    assert!(gate.cancel(), "there is a run to cancel");

    assert!(run.is_cancelled());
    assert!(gate.cancel(), "asking again is the same as asking once");
    assert!(run.is_cancelled());
}

#[test]
fn a_cancel_asked_through_a_clone_reaches_the_run() {
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("run");

    assert!(gate.clone().cancel());

    assert!(run.is_cancelled());
}

#[test]
fn a_cancel_with_no_run_is_refused_and_not_remembered() {
    let gate = SyncGate::new();

    assert!(!gate.cancel(), "nothing to cancel");

    let run = gate.try_begin().expect("run");
    assert!(!run.is_cancelled(), "the cancel did not wait for the next run");
}

#[test]
fn a_cancel_does_not_outlive_its_run() {
    let gate = SyncGate::new();
    let first = gate.try_begin().expect("first run");
    assert!(gate.cancel());
    drop(first);

    let second = gate.try_begin().expect("second run");

    assert!(!second.is_cancelled(), "the next run starts uncancelled");
    assert!(gate.try_begin().is_none(), "a cancelled run still holds the gate until it ends");
}

#[test]
fn a_cancel_does_not_open_the_gate() {
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("run");

    gate.cancel();

    assert!(gate.try_begin().is_none(), "the run holds the gate until it has stopped");
    drop(run);
    assert!(gate.try_begin().is_some());
}

#[test]
fn separate_gates_do_not_cancel_each_other() {
    let (a, b) = (SyncGate::new(), SyncGate::new());
    let run_a = a.try_begin().expect("run on a");
    let run_b = b.try_begin().expect("run on b");

    assert!(a.cancel());

    assert!(run_a.is_cancelled());
    assert!(!run_b.is_cancelled());
}
