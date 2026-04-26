## Nightly Tier-1 Corpus Failure

The nightly tier-1 corpus workflow failed.

- Workflow: `${{ github.workflow }}`
- Run id: `${{ github.run_id }}`
- Run url: https://github.com/${{ github.repository }}/actions/runs/${{ github.run_id }}
- Ref: `${{ github.ref }}`
- SHA: `${{ github.sha }}`

### What to check

1. Download and inspect the `corpus-report-*` artifact from the failed run.
2. Confirm whether the failure is a real regression, vendor/tooling drift, or environment noise.
3. If this is a true regression, open a follow-up fix PR and link it here.

### Notes

- This issue is updated by scheduled nightly failures.
- `workflow_dispatch` runs intentionally do not create/update this issue.
