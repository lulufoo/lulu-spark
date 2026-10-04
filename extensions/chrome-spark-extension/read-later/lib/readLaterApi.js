import { SPARK_BASE } from './config.js';

export async function save({ url, title }) {
  const endpoint = `${SPARK_BASE}/read-later`;

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

    return {
      ok: false,
      status: response.status,
      ...(typeof data.code === 'string' ? { code: data.code } : {}),
      error: data.error || `HTTP ${response.status}`,
      ...(data.entry ? { entry: data.entry } : {}),
    };
  } catch (err) {
    return {
      ok: false,
      status: 0,
      error: err?.message || 'Network error',
    };
  }
}
