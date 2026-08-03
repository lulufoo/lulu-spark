//! T5 closing-gate status for Cursor SDK production patch.
//!
//! Distinguishes contract-pass (test doubles allowed) from live-pass /
//! pending external acceptance. Mock evidence alone never satisfies Cursor AC.

use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractVerdict {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LiveSmokeVerdict {
    Pass,
    Fail,
    SkippedNoCredentials,
    SkippedEnvNotReady,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorAcVerdict {
    SatisfiedByLive,
    PendingExternalAcceptance,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosingGateReport {
    pub contract: ContractVerdict,
    pub live: LiveSmokeVerdict,
    pub cursor_ac: CursorAcVerdict,
    pub marks_cursor_ac_from_mock: bool,
    pub runner_installable: bool,
    pub runner_startable: bool,
    pub cursor_route_reachable: bool,
    pub endpoint_health_ok: bool,
    pub notes: String,
}

#[derive(Debug, Default)]
struct GateState {
    contract_pass: Option<bool>,
    live: LiveSmokeVerdict,
    mock_evidence_only: bool,
    runner_installable: bool,
    runner_startable: bool,
    cursor_route_reachable: bool,
    endpoint_health_ok: bool,
}

fn state() -> &'static Mutex<GateState> {
    static STATE: OnceLock<Mutex<GateState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(GateState::default()))
}

pub fn reset_for_tests() {
    let mut g = state().lock().unwrap_or_else(|e| e.into_inner());
    *g = GateState::default();
}

pub fn record_contract_pass(pass: bool) {
    state().lock().unwrap_or_else(|e| e.into_inner()).contract_pass = Some(pass);
}

pub fn record_live_smoke(verdict: LiveSmokeVerdict) {
    state().lock().unwrap_or_else(|e| e.into_inner()).live = verdict;
}

pub fn record_mock_evidence_only(v: bool) {
    state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .mock_evidence_only = v;
}

pub fn record_runner_installable(v: bool) {
    state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .runner_installable = v;
}

pub fn record_runner_startable(v: bool) {
    state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .runner_startable = v;
}

pub fn record_cursor_route_reachable(v: bool) {
    state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .cursor_route_reachable = v;
}

pub fn record_endpoint_health(v: bool) {
    state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .endpoint_health_ok = v;
}

pub fn evaluate_closing_gate() -> ClosingGateReport {
    let g = state().lock().unwrap_or_else(|e| e.into_inner());
    let contract = match g.contract_pass {
        Some(true) => ContractVerdict::Pass,
        Some(false) => ContractVerdict::Fail,
        None => ContractVerdict::Unknown,
    };
    let live = g.live;

    // Mock MUST NOT mark Cursor AC satisfied (L09-I #5).
    let marks_cursor_ac_from_mock = false;
    let cursor_ac = if g.mock_evidence_only {
        CursorAcVerdict::PendingExternalAcceptance
    } else {
        match live {
            LiveSmokeVerdict::Pass => CursorAcVerdict::SatisfiedByLive,
            LiveSmokeVerdict::Fail => CursorAcVerdict::Failed,
            LiveSmokeVerdict::SkippedNoCredentials
            | LiveSmokeVerdict::SkippedEnvNotReady
            | LiveSmokeVerdict::Unknown => CursorAcVerdict::PendingExternalAcceptance,
        }
    };

    let mut notes = String::new();
    notes.push_str(match contract {
        ContractVerdict::Pass => "contract: 合同通过 (ContractVerdict::Pass)\n",
        ContractVerdict::Fail => "contract: fail\n",
        ContractVerdict::Unknown => "contract: unknown\n",
    });
    notes.push_str(&format!("live: {live:?}\n"));
    match cursor_ac {
        CursorAcVerdict::SatisfiedByLive => {
            notes.push_str("cursor_ac: SatisfiedByLive\n");
        }
        CursorAcVerdict::PendingExternalAcceptance => {
            notes.push_str("cursor_ac: PendingExternalAcceptance / 外部验收待完成\n");
        }
        CursorAcVerdict::Failed => notes.push_str("cursor_ac: Failed\n"),
        CursorAcVerdict::Unknown => notes.push_str("cursor_ac: Unknown\n"),
    }
    if g.mock_evidence_only {
        notes.push_str("mock_evidence_only: true (does not satisfy Cursor AC)\n");
    }

    ClosingGateReport {
        contract,
        live,
        cursor_ac,
        marks_cursor_ac_from_mock,
        runner_installable: g.runner_installable,
        runner_startable: g.runner_startable,
        cursor_route_reachable: g.cursor_route_reachable,
        endpoint_health_ok: g.endpoint_health_ok,
        notes,
    }
}

pub fn closing_gate_markdown(report: &ClosingGateReport) -> String {
    format!(
        "# Cursor SDK Patch Closing Gate\n\n\
         Official SDK: https://cursor.com/docs/sdk/typescript\n\n\
         ## Verdicts\n\n\
         - contract: {:?}\n\
         - live: {:?}\n\
         - cursor_ac: {:?}\n\
         - marks_cursor_ac_from_mock: {}\n\n\
         ## Infrastructure checks\n\n\
         - runner_installable: {}\n\
         - runner_startable: {}\n\
         - cursor_route_reachable: {}\n\
         - endpoint_health_ok: {}\n\n\
         ## Notes\n\n\
         {}\n",
        report.contract,
        report.live,
        report.cursor_ac,
        report.marks_cursor_ac_from_mock,
        report.runner_installable,
        report.runner_startable,
        report.cursor_route_reachable,
        report.endpoint_health_ok,
        report.notes
    )
}
