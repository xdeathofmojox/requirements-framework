'use strict';
(() => {
  const key = 'requirements-viewer-theme';
  const system = matchMedia('(prefers-color-scheme: dark)');
  let preference = 'system';
  try {
    const saved = localStorage.getItem(key);
    if (saved === 'light' || saved === 'dark') preference = saved;
  } catch { /* Appearance still works when browser storage is unavailable. */ }
  function isDark() { return preference === 'dark' || (preference === 'system' && system.matches); }
  function apply() {
    if (preference === 'system') document.documentElement.removeAttribute('data-theme');
    else document.documentElement.setAttribute('data-theme', preference);
    const button = document.querySelector('#theme-toggle');
    if (!button) return;
    const dark = isDark();
    const label = dark ? 'Dark mode. Switch to light mode' : 'Light mode. Switch to dark mode';
    button.dataset.mode = dark ? 'dark' : 'light';
    button.setAttribute('aria-label', label);
    button.setAttribute('aria-pressed', String(dark));
    button.title = label;
  }
  apply();
  system.addEventListener('change', apply);
  document.addEventListener('DOMContentLoaded', () => {
    apply();
    document.querySelector('#theme-toggle').addEventListener('click', () => {
      preference = isDark() ? 'light' : 'dark';
      apply();
      try { localStorage.setItem(key, preference); }
      catch { /* Keep the current session's choice even if storage is blocked. */ }
    });
  });
})();
