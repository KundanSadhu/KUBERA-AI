/*
 * Renderer-side policy mirror for the Rust OfficeCLI adapter.
 * The backend remains authoritative; this module only prevents UI mistakes.
 */
const capabilities = {
  pptx: 'create_presentation',
  docx: 'create_document',
  xlsx: 'create_spreadsheet'
};

const actionByType = {
  pptx: ['create_presentation', 'render', 'dump'],
  docx: ['create_document', 'merge_template', 'render', 'dump'],
  xlsx: ['create_spreadsheet', 'render', 'dump']
};

export function validateOfficeRequest({ action, fileType, outputName, title = '', sections = [] }) {
  if (!actionByType[fileType]?.includes(action)) throw new Error('Office action is not permitted for the requested output type.');
  if (!/^[a-z0-9][a-z0-9._-]{0,179}\.(pptx|docx|xlsx)$/i.test(outputName || '')) throw new Error('Output name must be a safe Office filename.');
  if (!outputName.toLowerCase().endsWith(`.${fileType}`)) throw new Error('Output extension must match the selected file type.');
  if (title.length > 240 || sections.length > 25) throw new Error('Office request exceeds supported document limits.');
  return { requiredCapability: capabilities[fileType], action, fileType, outputName, title, sections };
}

export function capabilityAllows(agent, request) {
  return Array.isArray(agent?.capabilities) && agent.capabilities.includes(request.requiredCapability);
}
