//! Validator runner.

use crate::validator::{catalog, CheckCtx, Lens, LensDecision, Report, SegmentTree};

/// Run all enabled checks over a parsed tree.
pub fn run(tree: SegmentTree, lens: Lens) -> Report {
    let ctx = CheckCtx { lens };
    let mut findings = tree.diagnostics.clone();
    for check in catalog::checks() {
        let decision = lens.decide(check.id(), check.severity());
        let LensDecision::Emit(severity) = decision else {
            continue;
        };
        for mut finding in check.run(&ctx, &tree) {
            finding.severity = severity;
            findings.push(finding);
        }
    }
    findings.sort_by_key(|finding| (finding.byte_offset, finding.check_id));
    Report { findings }
}
