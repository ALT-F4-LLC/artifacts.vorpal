- Symptom: an implementation worker stalls after partially applying a scoped edit, and `git diff --stat` under-reports the worker's claimed doc change because the doc is untracked.
  Root cause: the worker did not reach a final report before close, while the close gate initially relied on tracked-diff evidence that omitted untracked files.
  Resolution: after one status probe, verify Docket state plus `git status --short <scoped paths>` before respawning; treat untracked scoped docs as first-class evidence during close gates and review briefs.
