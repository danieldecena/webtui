(() => {
  const SKIP = new Set([
    'SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE', 'SVG', 'CANVAS', 'IFRAME',
    'HEAD', 'META', 'LINK', 'AUDIO', 'VIDEO', 'OBJECT',
  ]);
  const HEADINGS = { H1: 1, H2: 2, H3: 3, H4: 4, H5: 5, H6: 6 };

  const blocks = [];
  let nextId = 1;
  let buf = '';

  const clean = (s) => (s || '').replace(/\s+/g, ' ').trim();

  // Bare separators between nav links carry nothing once the layout is gone.
  const SEPARATOR = /^[|·•\/\\\-–—]+$/;

  const flush = () => {
    const text = clean(buf);
    buf = '';
    if (text && !SEPARATOR.test(text)) {
      blocks.push({ kind: 'para', text, id: null, href: null, level: null });
    }
  };

  const push = (kind, text, el, extra = {}) => {
    let id = null;
    if (el) {
      id = nextId++;
      el.setAttribute('data-webtui-id', String(id));
    }
    blocks.push({ kind, text: clean(text), id, href: null, level: null, ...extra });
  };

  const visible = (el) => {
    const style = getComputedStyle(el);
    if (style.display === 'none' || style.visibility === 'hidden') return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 || r.height > 0;
  };

  const interactive = (el) =>
    (el.tagName === 'A' && el.hasAttribute('href')) ||
    el.tagName === 'BUTTON' ||
    el.tagName === 'INPUT' ||
    el.tagName === 'TEXTAREA' ||
    el.tagName === 'SELECT' ||
    el.getAttribute('role') === 'button' ||
    el.getAttribute('role') === 'link';

  const emitInteractive = (el) => {
    const tag = el.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA') {
      const type = (el.getAttribute('type') || 'text').toLowerCase();
      if (type === 'hidden') return;
      const label = el.getAttribute('placeholder') || el.getAttribute('aria-label') ||
        el.getAttribute('name') || type;
      push('input', el.value ? `${label}: ${el.value}` : label, el);
      return;
    }
    if (tag === 'SELECT') {
      push('input', el.options[el.selectedIndex]?.text || 'select', el);
      return;
    }
    const img = el.querySelector('img');
    const text = el.innerText || el.getAttribute('aria-label') || el.getAttribute('title') ||
      (img && (img.alt || img.title)) || '';
    if (tag === 'A') {
      push('link', text || '(icon)', el, { href: el.href });
      return;
    }
    push('button', text || 'button', el);
  };

  // An inline wrapper is folded into the surrounding text run, unless it hides
  // something clickable — then we have to descend to keep the link addressable.
  const hasInteractive = (el) =>
    el.querySelector('a[href], button, input, textarea, select, [role="button"], [role="link"]') !== null;

  const walk = (node) => {
    for (const child of node.childNodes) {
      if (child.nodeType === Node.TEXT_NODE) {
        buf += ' ' + child.textContent;
        continue;
      }
      if (child.nodeType !== Node.ELEMENT_NODE) continue;

      const el = child;
      if (SKIP.has(el.tagName) || !visible(el)) continue;

      if (interactive(el)) {
        flush();
        emitInteractive(el);
        continue;
      }

      const level = HEADINGS[el.tagName];
      if (level && !hasInteractive(el)) {
        flush();
        blocks.push({ kind: 'heading', text: clean(el.innerText), id: null, href: null, level });
        continue;
      }

      if (el.tagName === 'TR' && !hasInteractive(el)) {
        flush();
        const cells = [...el.children].map((c) => clean(c.innerText)).filter(Boolean);
        if (cells.length) {
          blocks.push({ kind: 'row', text: cells.join(' | '), id: null, href: null, level: null });
        }
        continue;
      }

      if (el.tagName === 'PRE' && !hasInteractive(el)) {
        flush();
        blocks.push({ kind: 'code', text: el.innerText.trim(), id: null, href: null, level: null });
        continue;
      }

      const inline = getComputedStyle(el).display.startsWith('inline');
      if (inline && !hasInteractive(el)) {
        buf += ' ' + el.innerText;
        continue;
      }

      if (el.tagName === 'LI') {
        flush();
        const before = blocks.length;
        walk(el);
        flush();
        // A list item whose whole payload was one paragraph reads better as a bullet.
        if (blocks.length === before + 1 && blocks[before].kind === 'para') {
          blocks[before].kind = 'listitem';
        }
        continue;
      }

      flush();
      walk(el);
      flush();
    }
  };

  document.querySelectorAll('[data-webtui-id]').forEach((el) => el.removeAttribute('data-webtui-id'));
  walk(document.body);
  flush();

  return { url: location.href, title: document.title, blocks };
})()
