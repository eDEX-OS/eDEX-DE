/* ============================================================
   eDEX-DE — docs/assets/js/main.js
   Nav toggle, active link, copy buttons, platform tabs,
   FAQ accordion, TOC highlight, smooth scroll, theme toggle
   ============================================================ */

'use strict';

/* ---- Nav mobile toggle ---- */
(function () {
  const toggle = document.getElementById('navToggle');
  const links  = document.getElementById('navLinks');
  if (!toggle || !links) return;
  toggle.addEventListener('click', () => {
    links.classList.toggle('open');
    toggle.setAttribute('aria-expanded', links.classList.contains('open'));
  });
  document.addEventListener('click', e => {
    if (!toggle.contains(e.target) && !links.contains(e.target)) {
      links.classList.remove('open');
    }
  });
})();

/* ---- Active nav link ---- */
(function () {
  const path = window.location.pathname;
  document.querySelectorAll('.nav-links a').forEach(a => {
    const href = a.getAttribute('href');
    if (href && (path.endsWith(href.replace(/^\.\.\//, '').replace(/index\.html$/, '')) ||
                 (path.endsWith('/') && href === 'index.html'))) {
      a.classList.add('active');
    }
  });
})();

/* ---- Copy code buttons ---- */
(function () {
  document.querySelectorAll('.code-header').forEach(header => {
    const btn = header.querySelector('.copy-btn');
    if (!btn) return;
    btn.addEventListener('click', () => {
      const pre = header.nextElementSibling;
      if (!pre) return;
      const text = pre.innerText || pre.textContent;
      navigator.clipboard.writeText(text).then(() => {
        btn.textContent = 'Copied!';
        btn.classList.add('copied');
        setTimeout(() => { btn.textContent = 'Copy'; btn.classList.remove('copied'); }, 2000);
      });
    });
  });
})();

/* ---- Platform tabs ---- */
(function () {
  document.querySelectorAll('.platform-tabs').forEach(container => {
    const buttons = container.querySelectorAll('.tab-btn');
    const panels  = container.querySelectorAll('.tab-panel');
    buttons.forEach(btn => {
      btn.addEventListener('click', () => {
        const target = btn.dataset.tab;
        buttons.forEach(b => b.classList.remove('active'));
        panels.forEach(p => p.classList.remove('active'));
        btn.classList.add('active');
        const panel = container.querySelector(`.tab-panel[data-tab="${target}"]`);
        if (panel) panel.classList.add('active');
      });
    });
    if (buttons.length) buttons[0].click();
  });
})();

/* ---- FAQ accordion ---- */
(function () {
  document.querySelectorAll('.faq-question').forEach(btn => {
    btn.addEventListener('click', () => {
      const item = btn.closest('.faq-item');
      const isOpen = item.classList.contains('open');
      document.querySelectorAll('.faq-item.open').forEach(i => i.classList.remove('open'));
      if (!isOpen) item.classList.add('open');
    });
  });
})();

/* ---- TOC active section highlight ---- */
(function () {
  const tocLinks = document.querySelectorAll('.toc a[href^="#"]');
  if (!tocLinks.length) return;
  const observer = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const id = entry.target.getAttribute('id');
        tocLinks.forEach(a => {
          a.classList.toggle('active', a.getAttribute('href') === `#${id}`);
        });
      }
    });
  }, { rootMargin: '-15% 0px -75% 0px' });
  tocLinks.forEach(a => {
    const id = a.getAttribute('href').slice(1);
    const el = document.getElementById(id);
    if (el) observer.observe(el);
  });
})();

/* ---- Smooth scroll ---- */
document.querySelectorAll('a[href^="#"]').forEach(a => {
  a.addEventListener('click', e => {
    const id = a.getAttribute('href').slice(1);
    const el = document.getElementById(id);
    if (el) { e.preventDefault(); el.scrollIntoView({ behavior: 'smooth', block: 'start' }); }
  });
});
