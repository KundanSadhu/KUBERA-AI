export const escapeHtml = (value = '') => String(value)
  .replaceAll('&', '&amp;')
  .replaceAll('<', '&lt;')
  .replaceAll('>', '&gt;')
  .replaceAll('"', '&quot;')
  .replaceAll("'", '&#039;');

export const formatCurrency = (value) => new Intl.NumberFormat('en-IN', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 0
}).format(Number(value || 0));

export const formatDateTime = (value) => new Intl.DateTimeFormat('en-IN', {
  dateStyle: 'medium',
  timeStyle: 'short'
}).format(new Date(value));

export const formatDate = (value) => new Intl.DateTimeFormat('en-IN', {
  dateStyle: 'medium'
}).format(new Date(value));

export const relativeTime = (value) => {
  const seconds = Math.max(0, Math.floor((Date.now() - Number(value)) / 1000));
  if (seconds < 8) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
};

export const id = (prefix) => `${prefix}-${globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random().toString(16).slice(2)}`}`;

export const typeFromPrompt = (text) => {
  const value = text.toLowerCase();
  if (/spreadsheet|budget|tracker|workbook|xlsx/.test(value)) return 'xlsx';
  if (/report|docx|template|letter/.test(value)) return 'docx';
  return 'pptx';
};

export const fileIcon = (type) => ({ pptx: '▱', docx: '≡', xlsx: '▦', pdf: '▤' }[type] || '▱');

export const statusTone = (status) => {
  if (['done', 'approved', 'succeeded', 'ready', 'active'].includes(status)) return 'success';
  if (['running', 'in_progress', 'planning', 'queued', 'review'].includes(status)) return 'info';
  if (['blocked', 'paused'].includes(status)) return 'warning';
  if (['failed', 'error', 'cancelled', 'offline'].includes(status)) return 'danger';
  return 'neutral';
};
