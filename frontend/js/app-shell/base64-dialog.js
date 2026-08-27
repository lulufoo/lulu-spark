document.getElementById('btn-base64-encode').addEventListener('click', () => {
  const input = document.getElementById('base64-input').value;
  const bytes = new TextEncoder().encode(input);
  const result = btoa(String.fromCharCode(...bytes));
  document.getElementById('base64-output').value = result;
  document.getElementById('base64-output').style.color = '';
});

document.getElementById('btn-base64-decode').addEventListener('click', () => {
  try {
    const input = document.getElementById('base64-input').value;
    const result = new TextDecoder().decode(
      Uint8Array.from(atob(input), c => c.charCodeAt(0))
    );
    document.getElementById('base64-output').value = result;
    document.getElementById('base64-output').style.color = '';
  } catch {
    document.getElementById('base64-output').value = '⚠️ Decode failed: input is not valid Base64';
    document.getElementById('base64-output').style.color = '#e5534b';
  }
});

document.getElementById('btn-base64-copy').addEventListener('click', () => {
  const text = document.getElementById('base64-output').value;
  if (!text) return;
  navigator.clipboard.writeText(text);
  const btn = document.getElementById('btn-base64-copy');
  btn.textContent = 'Copied';
  setTimeout(() => { btn.textContent = 'Copy result'; }, 1500);
});
