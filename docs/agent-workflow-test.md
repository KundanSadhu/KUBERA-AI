# Managed Workforce Test

## Automated local test

Run:

```bash
npm run test:agents
```

The test executes a full Acme board-deck workforce run without browser timers or a provider dependency.

## Assertions

- Asha creates the manager plan.
- Maya creates a source-linked `market_analysis` handoff.
- W-1 creates a source-linked `pipeline_metrics` handoff.
- Cross-peer review accepts both handoffs.
- W-2 creates an artifact plan only after accepted dependencies.
- Sentinel approves the artifact after checking type, outline, and provenance.
- The resulting Acme report has a `$365,000` pipeline value and three linked source records.

## Manual UI test

1. Run `npm run dev`.
2. Submit `Build a Q3 pipeline deck for Acme Corp`.
3. Open **Workforce** while the mission is running.
4. Observe:
   - Maya and W-1 in parallel in **Working**.
   - Both tasks transition to **Peer review**.
   - The document worker stays **Queued** until both handoffs are accepted.
   - W-2 moves to **Working**, then **Delivered**.
   - Sentinel moves the final file to **Approved**.
5. Open Reports and inspect the provenance-aware artifact preview.
