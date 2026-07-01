import { WORKBENCH_BASE } from './config.js';

export async function save({ url, title }) {
  const endpoint = `${WORKBENCH_BASE}/api/read-later`;

  try {
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url, title }),
    });

    let data = {};
    try {
      data = await response.json();
    } catch {
      data = {};
    }

    if (response.status === 201) {
      return { ok: true, status: 201, entry: data };
    }

    if (response.status === 503) {
      return {
        ok: false,
        status: 503,
        error: data.error || 'Workbench not running',
      };
    }

    return {
      ok: false,
      status: response.status,
      error: data.error || `HTTP ${response.status}`,
    };
  } catch (err) {
    return {
      ok: false,
      status: 0,
      error: err?.message || 'Network error',
    };
  }
}
