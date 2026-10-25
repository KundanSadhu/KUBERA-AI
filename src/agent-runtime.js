/*
 * Local, deterministic workforce runtime.
 *
 * This is deliberately useful without a provider: every worker operates on the
 * selected business records and produces structured, source-linked output.
 * The OpenCode adapter can replace an individual role later, but the task and
 * handoff contract remains the same.
 */

const money = (value) => new Intl.NumberFormat('en-US', {
  style: 'currency', currency: 'USD', maximumFractionDigits: 0
}).format(Number(value || 0));

function titleCase(text) {
  return text.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function selectBusinessContext(state, requirementText) {
  const prompt = requirementText.toLowerCase();
  const matchedClient = state.clients.find((client) => prompt.includes(client.name.toLowerCase()));
  const activeClientIds = new Set(state.clients.filter((client) => client.status === 'active').map((client) => client.id));
  const projects = matchedClient
    ? state.projects.filter((project) => project.clientId === matchedClient.id)
    : state.projects.filter((project) => activeClientIds.has(project.clientId));
  const clients = matchedClient ? [matchedClient] : state.clients.filter((client) => activeClientIds.has(client.id));
  return {
    client: matchedClient || null,
    clients,
    projects,
    sourceRecordIds: [...clients.map((client) => client.id), ...projects.map((project) => project.id)]
  };
}

export function planWorkforce({ requirementText, fileType, context }) {
  const scope = context.client?.name || `${context.clients.length} active clients`;
  return {
    title: `${scope} ${fileType.toUpperCase()} mission`,
    deliverableType: fileType,
    recordSelector: context.client ? { clientName: context.client.name, includeProjects: true } : { includeProjects: true, status: 'active' },
    tasks: [
      { key: 'market_analysis', agentId: 'agent-market', reviewerId: 'agent-worker-1', title: 'Assess opportunity and risk signals', handoffType: 'market_analysis', dependsOn: [] },
      { key: 'pipeline_metrics', agentId: 'agent-worker-1', reviewerId: 'agent-market', title: 'Prepare pipeline metrics', handoffType: 'pipeline_metrics', dependsOn: [] },
      { key: 'document', agentId: 'agent-worker-2', reviewerId: 'agent-reviewer', title: `Create ${fileType.toUpperCase()} deliverable`, handoffType: 'final_document', dependsOn: ['market_analysis', 'pipeline_metrics'] },
      { key: 'review', agentId: 'agent-reviewer', reviewerId: null, title: 'Validate final artifact', handoffType: 'validation', dependsOn: ['document'] }
    ],
    summary: `Bounded four-worker plan for ${scope}; ${context.projects.length} project records selected.`
  };
}

export function runMarketAnalyst(context) {
  const risks = context.projects.filter((project) => project.risk);
  const largest = [...context.projects].sort((a, b) => b.value - a.value)[0];
  const findings = [];
  if (largest) {
    findings.push({
      claim: `${largest.name} is the largest selected opportunity at ${money(largest.value)}.`,
      evidenceRecordIds: [largest.id],
      confidence: 'high'
    });
  }
  for (const project of risks) {
    findings.push({
      claim: `${project.name} is flagged at risk and requires executive attention.`,
      evidenceRecordIds: [project.id],
      confidence: 'high'
    });
  }
  if (!findings.length && context.clients.length) {
    findings.push({
      claim: 'No project opportunity or risk records were available for additional analysis.',
      evidenceRecordIds: context.clients.map((client) => client.id),
      confidence: 'high'
    });
  }
  return {
    handoffType: 'market_analysis',
    executiveSummary: risks.length
      ? `${risks.length} at-risk initiative${risks.length === 1 ? '' : 's'} identified across ${context.projects.length} selected project records.`
      : `No selected projects are flagged at risk across ${context.projects.length} project records.`,
    findings,
    recommendedFocus: risks.length
      ? ['Resolve the flagged delivery risk before the next executive review.']
      : ['Maintain progression of the current selected opportunities.'],
    evidenceRecordIds: context.sourceRecordIds
  };
}

export function runOperationsAnalyst(context) {
  const stageMap = new Map();
  let total = 0;
  let atRisk = 0;
  context.projects.forEach((project) => {
    total += Number(project.value || 0);
    if (project.risk) atRisk += 1;
    const stage = project.stage || 'Unstaged';
    const item = stageMap.get(stage) || { stage, count: 0, value: 0 };
    item.count += 1;
    item.value += Number(project.value || 0);
    stageMap.set(stage, item);
  });
  return {
    handoffType: 'pipeline_metrics',
    totalPipelineValue: total,
    projectCount: context.projects.length,
    atRiskCount: atRisk,
    stageBreakdown: [...stageMap.values()].sort((a, b) => b.value - a.value),
    evidenceRecordIds: context.projects.map((project) => project.id)
  };
}

export function reviewHandoff(handoff) {
  const issues = [];
  if (!handoff?.handoffType) issues.push('Handoff type is missing.');
  if (!Array.isArray(handoff?.evidenceRecordIds) || handoff.evidenceRecordIds.length === 0) issues.push('Handoff has no source record references.');
  if (handoff?.handoffType === 'pipeline_metrics' && typeof handoff.totalPipelineValue !== 'number') issues.push('Pipeline metrics must contain a numeric total.');
  if (handoff?.handoffType === 'market_analysis' && !Array.isArray(handoff.findings)) issues.push('Market analysis must contain findings.');
  return { approved: issues.length === 0, issues };
}

export function buildArtifact({ requirementText, fileType, context, marketAnalysis, pipelineMetrics }) {
  const scope = context.client?.name || 'Active portfolio';
  const title = titleCase(requirementText.trim().replace(/[.?!]+$/, ''));
  const riskProjects = context.projects.filter((project) => project.risk);
  const metric = `${money(pipelineMetrics.totalPipelineValue)} across ${pipelineMetrics.projectCount} projects`;
  let outline;
  if (fileType === 'docx') {
    outline = ['Executive summary', `${scope} account profile`, `Pipeline update — ${metric}`, riskProjects.length ? 'Risks and mitigation' : 'Delivery health', 'Next steps'];
  } else if (fileType === 'xlsx') {
    outline = ['Budget tracker summary', 'Project and stage rows', 'Pipeline value totals', 'At-risk work items', 'Variance / actions'];
  } else {
    outline = ['Executive overview', `Pipeline snapshot — ${metric}`, 'Active opportunities', riskProjects.length ? 'At-risk initiatives' : 'Delivery health', 'Recommended actions'];
  }
  return {
    title,
    outline,
    dataSummary: {
      scope,
      pipelineValue: pipelineMetrics.totalPipelineValue,
      projectCount: pipelineMetrics.projectCount,
      atRiskCount: pipelineMetrics.atRiskCount,
      recommendations: marketAnalysis.recommendedFocus
    },
    sourceRecordIds: context.sourceRecordIds,
    provenance: {
      marketAnalysisSources: marketAnalysis.evidenceRecordIds,
      pipelineMetricSources: pipelineMetrics.evidenceRecordIds
    }
  };
}

export function reviewArtifact({ report, fileType }) {
  const issues = [];
  if (!report?.name?.endsWith(`.${fileType}`)) issues.push('Artifact extension does not match expected deliverable type.');
  if (!report?.outline?.length) issues.push('Artifact requires a non-empty content outline.');
  if (!report?.sourceRecordIds?.length) issues.push('Artifact requires source provenance.');
  return { approved: issues.length === 0, issues };
}
