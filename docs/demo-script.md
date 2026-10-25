# Demo Script

## Preparation

1. Run the application and wait for the header to show **Engine: Ready** (or the prototype equivalent).
2. Open **Records** and confirm seeded data:
   - Acme Corp — Enterprise SaaS — Priya Shah
   - Q3 Platform Expansion — Proposal — $240,000
   - Analytics Modernization — Negotiation — $125,000 — At risk
3. Open **Settings** and confirm provider/model settings or simulated demo engine.
4. Confirm a client-report template is available if demonstrating DOCX merge.

## Demo 1 — Managed workforce to board deck

1. Open **Missions**.
2. Enter:

   ```text
   Build a Q3 pipeline deck for Acme Corp
   ```

3. Explain that KUBERA creates a durable business mission, rather than a chat answer.
4. Open the mission’s **Workforce** deployment panel.
5. Point out:
   - Asha plans the bounded team.
   - Maya and W-1 can work in parallel.
   - W-2 waits for accepted analysis/metric handoffs.
   - Sentinel validates the final file.
6. Expand **Activity** briefly and show record-query, deployment, handoff, document, and validation events.
7. When the mission finishes, open **Reports**.
8. Click **Open** in the desktop build, or inspect the prototype report row.
9. Open the PPTX in a native Office-compatible app. It must not trigger a repair prompt.

## Demo 2 — Template-backed client report

1. From **Missions**, enter:

   ```text
   Write a Q3 client report for Acme using the client report template
   ```

2. Show the manager choosing the required document path.
3. Open the generated DOCX from Report Center.
4. Point out resolved `{{CLIENT}}`, `{{QUARTER}}`, revenue/pipeline, owner, and risk values.

## Demo 3 — Schedule

1. Create/show a schedule with a business goal, output type, cron expression, and timezone.
2. Point out the enabled state and **Next run**.
3. State the MVP boundary: schedules execute while the desktop app remains open.
4. If practical, use a near-future test cron to show a `schedule-fired` mission being created.

## Demo 4 — Persistence and traceability

1. Restart the desktop app.
2. Show that Records, Missions, agent runs/handoffs, Reports, and Schedule rows remain.
3. Open a deck mission’s activity/audit trail.
4. Trace the artifact from W-2 back through the accepted Maya/W-1 handoffs to Acme and its project record IDs.

## Closing statement

> “KUBERA turns a business goal into a real Office file through a small, governed agent workforce. Every worker is constrained to its purpose, every handoff is traceable to local records, and the artifact—not chat prose—is the business result.”
