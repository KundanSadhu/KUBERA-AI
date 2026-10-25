const now = Date.now();
const ago = (minutes) => now - minutes * 60_000;

export function createSeedState() {
  const acme = 'client-acme';
  const deckMission = 'mission-acme-deck';
  const report = 'report-acme-q3-deck';

  return {
    version: 1,
    route: 'missions',
    engine: { status: 'ready', model: 'local deterministic', patrol: true, message: 'Local workforce runtime ready' },
    settings: {
      providerBaseUrl: 'https://api.example.com/v1',
      model: 'gpt-4o-mini',
      timezone: 'Asia/Calcutta'
    },
    clients: [
      { id: acme, name: 'Acme Corp', industry: 'Enterprise SaaS', status: 'active', owner: 'Priya Shah', createdAt: ago(7200) },
      { id: 'client-northstar', name: 'Northstar Health', industry: 'Healthcare', status: 'active', owner: 'Daniel Reed', createdAt: ago(6120) },
      { id: 'client-meridian', name: 'Meridian Retail', industry: 'Retail', status: 'prospect', owner: 'Priya Shah', createdAt: ago(5040) }
    ],
    projects: [
      { id: 'project-acme-expansion', clientId: acme, name: 'Q3 Platform Expansion', stage: 'Proposal', value: 240000, risk: false, createdAt: ago(840) },
      { id: 'project-acme-analytics', clientId: acme, name: 'Analytics Modernization', stage: 'Negotiation', value: 125000, risk: true, createdAt: ago(690) },
      { id: 'project-northstar-portal', clientId: 'client-northstar', name: 'Patient Portal Discovery', stage: 'Discovery', value: 80000, risk: false, createdAt: ago(480) }
    ],
    agents: [
      { id: 'agent-manager', name: 'Asha', label: 'Manager', kind: 'manager', capability: 'Plan subagents', status: 'idle', color: 'violet' },
      { id: 'agent-market', name: 'Maya', label: 'Market Analyst', kind: 'analyst', capability: 'Analyze market context', status: 'idle', color: 'blue' },
      { id: 'agent-worker-1', name: 'W-1', label: 'Operations Analyst', kind: 'worker', capability: 'Prepare pipeline metrics', status: 'idle', color: 'teal' },
      { id: 'agent-worker-2', name: 'W-2', label: 'Document Worker', kind: 'worker', capability: 'Create Office artifacts', status: 'idle', color: 'amber' },
      { id: 'agent-reviewer', name: 'Sentinel', label: 'File Reviewer', kind: 'reviewer', capability: 'Validate output', status: 'idle', color: 'rose' }
    ],
    missions: [
      {
        id: deckMission,
        text: 'Build a Q3 pipeline deck for Acme Corp',
        title: 'Acme Corp Q3 pipeline deck',
        type: 'pptx',
        status: 'done',
        createdAt: ago(135),
        updatedAt: ago(128),
        reportId: report,
        tasks: [
          { id: 'task-manager-seed', agentId: 'agent-manager', title: 'Plan a bounded delivery team', status: 'succeeded', dependsOn: [], summary: 'Deployed two analysts, a document worker, and reviewer.' },
          { id: 'task-market-seed', agentId: 'agent-market', title: 'Assess opportunity and risk signals', status: 'succeeded', dependsOn: [], summary: 'Found one delivery risk and two executive themes.' },
          { id: 'task-ops-seed', agentId: 'agent-worker-1', title: 'Prepare pipeline metrics', status: 'succeeded', dependsOn: [], summary: 'Reconciled $365,000 pipeline across two projects.' },
          { id: 'task-doc-seed', agentId: 'agent-worker-2', title: 'Create board deck', status: 'succeeded', dependsOn: ['task-market-seed', 'task-ops-seed'], summary: 'Created 5-slide board-ready presentation.' },
          { id: 'task-review-seed', agentId: 'agent-reviewer', title: 'Validate PPTX artifact', status: 'approved', dependsOn: ['task-doc-seed'], summary: 'Verified expected type and report registration.' }
        ],
        handoffs: [
          { id: 'handoff-market-seed', fromAgentId: 'agent-market', type: 'market_analysis', sourceCount: 2, status: 'accepted' },
          { id: 'handoff-ops-seed', fromAgentId: 'agent-worker-1', type: 'pipeline_metrics', sourceCount: 2, status: 'accepted' }
        ]
      }
    ],
    reports: [
      {
        id: report,
        missionId: deckMission,
        name: 'acme-corp_q3-pipeline-board-deck_2026-10-25.pptx',
        type: 'pptx',
        createdAt: ago(128),
        size: '2.4 MB',
        producer: 'W-2 — Document Worker',
        prototype: true,
        outline: ['Executive overview', 'Pipeline snapshot — $365,000', 'Active opportunities', 'At-risk initiative', 'Recommended actions']
      }
    ],
    templates: [
      { id: 'template-client-q3', name: 'Client Q3 account report', fileName: 'client-q3-account-report.docx', type: 'docx', placeholders: 7, createdAt: ago(10080) }
    ],
    schedules: [
      { id: 'schedule-monthly-acme', prompt: 'Build the monthly Acme pipeline board deck from current records', type: 'pptx', cron: '0 9 1 * *', timezone: 'Asia/Calcutta', enabled: true, lastRun: ago(43800), nextRun: now + 5 * 24 * 60 * 60_000 }
    ],
    activity: [
      { id: 'audit-1', at: ago(128), kind: 'validate_output', tone: 'success', text: 'Sentinel approved Acme’s PPTX report artifact.' },
      { id: 'audit-2', at: ago(129), kind: 'create_file', tone: 'info', text: 'W-2 created the Q3 pipeline deck in the Report Center.' },
      { id: 'audit-3', at: ago(131), kind: 'consume_handoff', tone: 'info', text: 'W-2 consumed market_analysis and pipeline_metrics handoffs.' },
      { id: 'audit-4', at: ago(133), kind: 'query_projects', tone: 'neutral', text: 'Maya and W-1 used 2 Acme project records for the mission.' }
    ]
  };
}
