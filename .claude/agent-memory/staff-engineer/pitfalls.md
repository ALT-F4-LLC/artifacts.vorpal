# Staff-Engineer Recurring Pitfalls

Append-only ledger of recurring review/design pitfalls in `symptom → root cause → resolution` form. Harvested by evolve-* cycles. Never edit/remove prior entries.

---

- **Regression guard whose falsifier was never confirmed** — symptom: a fix ships with a "smoke test" / regression test that asserts the SUCCESS path (exits 0, no error string) and is reported validated because the positive case passes. → root cause: the test's value depends on it actually exercising the failure path, which was never proven — the author confirmed "it passes when fixed" but not "it fails when broken." A success-only validation cannot distinguish a real guard from a no-op (false negative). Compounded when a script `echo` ("OK: launched past initTheme") is mistaken for evidence the asserted condition was reached. → resolution: in review, demand the discriminating measurement (TFD-4) — run the EXACT test invocation against the KNOWN-BROKEN state and confirm it FAILS. Classify as a Concern (not Blocker) when the underlying fix is independently verified correct; the finding governs whether the guard guards, not whether the fix works. Cheap to confirm when the author already has both layouts.
