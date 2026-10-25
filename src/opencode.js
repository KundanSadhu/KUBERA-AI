/*
 * OpenCode loopback client.
 *
 * This module is intentionally separate from UI rendering. It is activated by
 * the desktop runtime only after Rust reports a healthy loopback sidecar.
 * Endpoint shapes must be verified against the pinned OpenCode sidecar version.
 */
// Browser code uses the same-origin development proxy. The proxy—not the browser—talks to the loopback sidecar.
const DEFAULT_BASE_URL = '/opencode';

export function createOpenCodeClient(baseUrl = DEFAULT_BASE_URL) {
  const base = baseUrl.replace(/\/$/, '');

  async function request(path, options = {}) {
    const response = await fetch(`${base}${path}`, {
      headers: { 'Content-Type': 'application/json', ...(options.headers || {}) },
      ...options
    });
    if (!response.ok) throw new Error(`OpenCode ${path} failed (${response.status})`);
    const text = await response.text();
    return text ? JSON.parse(text) : null;
  }

  return {
    async health() {
      return request('/global/health', { method: 'GET', headers: {} });
    },
    async createSession(payload = {}) {
      return request('/session', { method: 'POST', body: JSON.stringify(payload) });
    },
    async sendMessage(sessionId, payload) {
      if (!sessionId) throw new Error('OpenCode session ID is required.');
      return request(`/session/${encodeURIComponent(sessionId)}/message`, { method: 'POST', body: JSON.stringify(payload) });
    },
    subscribe(onEvent, onError) {
      const stream = new EventSource(`${base}/global/event`);
      stream.onmessage = (event) => {
        try { onEvent(JSON.parse(event.data)); } catch { onEvent({ type: 'unparsed', data: event.data }); }
      };
      stream.onerror = (error) => onError?.(error);
      return () => stream.close();
    }
  };
}

export function eventForSession(event, sessionId) {
  const candidate = event?.sessionID || event?.sessionId || event?.properties?.sessionID;
  return candidate === sessionId;
}
