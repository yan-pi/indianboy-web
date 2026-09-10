(() => {
  const root = document.documentElement;
  const themeButton = document.querySelector('[data-theme-toggle]');
  const media = matchMedia('(prefers-color-scheme: dark)');
  let preference = localStorage.getItem('theme');
  if (!['light', 'dark', 'system'].includes(preference || '')) preference = 'system';
  const applyTheme = () => {
    const resolved = preference === 'system' ? (media.matches ? 'dark' : 'light') : preference;
    root.dataset.theme = resolved;
    root.dataset.themePreference = preference;
    if (themeButton) themeButton.textContent = `theme: ${preference}`;
  };
  applyTheme();
  media.addEventListener('change', () => { if (preference === 'system') applyTheme(); });

  themeButton?.addEventListener('click', () => {
    preference = preference === 'light' ? 'dark' : preference === 'dark' ? 'system' : 'light';
    localStorage.setItem('theme', preference);
    applyTheme();
    document.querySelectorAll('iframe.giscus-frame').forEach((frame) => {
      frame.contentWindow?.postMessage({ giscus: { setConfig: { theme: root.dataset.theme === 'dark' ? 'dark' : 'light' } } }, 'https://giscus.app');
    });
  });

  document.querySelector('[data-back]')?.addEventListener('click', () => {
    if (history.length > 1) history.back();
    else location.href = '/blog';
  });

  const progress = document.querySelector('[data-reading-progress]');
  if (progress) {
    const updateProgress = () => {
      const maximum = document.documentElement.scrollHeight - innerHeight;
      progress.style.transform = `scaleX(${maximum > 0 ? scrollY / maximum : 0})`;
    };
    addEventListener('scroll', updateProgress, { passive: true });
    addEventListener('resize', updateProgress);
    updateProgress();
  }

  document.querySelector('[data-copy-url]')?.addEventListener('click', async (event) => {
    const button = event.currentTarget;
    try {
      await navigator.clipboard.writeText(location.href);
      button.textContent = 'copied';
      setTimeout(() => { button.textContent = 'copy link'; }, 2000);
    } catch { button.textContent = 'copy unavailable'; }
  });

  const filterToggle = document.querySelector('[data-filter-toggle]');
  const filterControls = document.querySelector('[data-filter-controls]');
  const filterRows = [...document.querySelectorAll('[data-post-list] [data-tags]')];
  const filterEmpty = document.querySelector('[data-filter-empty]');
  if (filterToggle && filterControls) {
    filterToggle.addEventListener('click', () => {
      const expanded = filterToggle.getAttribute('aria-expanded') === 'true';
      filterToggle.setAttribute('aria-expanded', String(!expanded));
      filterToggle.textContent = expanded ? 'show' : 'hide';
      filterControls.hidden = expanded;
    });
  }
  document.querySelectorAll('[data-filter-controls] [data-tag]').forEach((button) => {
    button.addEventListener('click', () => {
      const tag = button.dataset.tag || '';
      const selected = new Set([...document.querySelectorAll('[data-filter-controls] [aria-pressed="true"]')].map((item) => item.dataset.tag).filter(Boolean));
      if (tag === '') selected.clear();
      else if (selected.has(tag)) selected.delete(tag);
      else selected.add(tag);
      document.querySelectorAll('[data-filter-controls] [data-tag]').forEach((item) => {
        const active = item.dataset.tag === '' ? selected.size === 0 : selected.has(item.dataset.tag);
        item.setAttribute('aria-pressed', String(active));
      });
      let visible = 0;
      filterRows.forEach((row) => {
        const tags = (row.dataset.tags || '').split('|');
        const show = selected.size === 0 || tags.some((item) => selected.has(item));
        row.hidden = !show;
        if (show) visible += 1;
      });
      if (filterEmpty) filterEmpty.hidden = visible !== 0;
    });
  });

  const renderMath = () => {
    if (!window.katex) return;
    document.querySelectorAll('[data-math-style]').forEach((node) => {
      if (node.dataset.rendered) return;
      const display = node.dataset.mathStyle === 'display';
      const source = node.textContent || '';
      try {
        node.innerHTML = window.katex.renderToString(source, { displayMode: display, throwOnError: false });
        node.dataset.rendered = 'true';
      } catch { /* Keep source text readable if KaTeX cannot parse it. */ }
    });
  };
  const renderMermaid = async () => {
    const diagrams = [...document.querySelectorAll('pre code.language-mermaid')];
    if (diagrams.length === 0) return;
    try {
      const module = await import('https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs');
      const mermaid = module.default;
      mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', theme: root.dataset.theme === 'dark' ? 'dark' : 'default' });
      for (const [index, code] of diagrams.entries()) {
        const { svg } = await mermaid.render(`diagram-${index}`, code.textContent || '');
        code.closest('pre').outerHTML = `<div class="mermaid-diagram">${svg}</div>`;
      }
    } catch {
      // Mermaid remains as a readable highlighted code block when unavailable.
    }
  };
  window.addEventListener('load', () => { renderMath(); renderMermaid(); });
})();
