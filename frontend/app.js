const GW_BASE = (window.GW_BASE || localStorage.getItem('GW_BASE') || 'http://127.0.0.1:6188');

function getApiKey() {
  return localStorage.getItem('apiKey') || '';
}

function setApiKey(key) {
  localStorage.setItem('apiKey', key);
}

async function request(url) {
  const el = document.getElementById('output');
  const key = getApiKey();
  if (!key) {
    el.textContent = '需要先保存 API Key 才可请求';
    return;
  }
  el.textContent = '加载中...';
  try {
    const fullUrl = url.startsWith('http') ? url : `${GW_BASE}${url}`;
    const res = await fetch(fullUrl, { headers: { 'X-API-Key': key } });
    const data = await res.json();
    el.textContent = JSON.stringify(data, null, 2);
  } catch (err) {
    el.textContent = `请求失败: ${err}`;
  }
}

document.getElementById('fetchPosts').addEventListener('click', () => {
  request('/api/posts');
});

document.getElementById('fetchPost').addEventListener('click', () => {
  const id = document.getElementById('postId').value || '1';
  request(`/api/posts/${id}`);
});

// Health check
(async () => {
  const el = document.getElementById('health');
  try {
    const res = await fetch(`${GW_BASE}/health`);
    const data = await res.json();
    el.textContent = data.status;
  } catch {
    el.textContent = '失败';
  }
})();

// Init API Key input
(() => {
  const input = document.getElementById('apiKeyInput');
  const btn = document.getElementById('saveApiKey');
  if (input && btn) {
    input.value = getApiKey();
    btn.addEventListener('click', () => {
      const v = input.value.trim();
      setApiKey(v);
      const el = document.getElementById('output');
      el.textContent = v ? 'API Key 已保存' : '已清空 API Key';
    });
  }
})();