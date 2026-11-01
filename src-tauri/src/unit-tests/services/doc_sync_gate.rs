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
