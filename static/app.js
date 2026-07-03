function addProductUrlField() {
  const wrap = document.getElementById('product-url-fields');
  const input = document.createElement('input');
  input.type = 'url';
  input.name = 'product_url';
  input.placeholder = 'https://…';
  wrap.appendChild(input);
  input.focus();
}

// Show/hide the "current gross weight" field based on the weight-mode radios.
// Plain JS instead of a CSS :has() selector so it works on older mobile browsers too.
function initWeightToggles() {
  document.querySelectorAll('.weight-toggle').forEach((fieldset) => {
    const sync = () => {
      const opened = fieldset.querySelector('input[name="weight_mode"][value="opened"]');
      fieldset.classList.toggle('is-opened', !!(opened && opened.checked));
    };
    fieldset.querySelectorAll('input[name="weight_mode"]').forEach((input) => {
      input.addEventListener('change', sync);
    });
    sync();
  });
}

document.addEventListener('DOMContentLoaded', initWeightToggles);

// Click a spool photo to view it full-size; click the backdrop or close button to dismiss.
function initLightbox() {
  const lightbox = document.getElementById('lightbox');
  const lightboxImg = document.getElementById('lightbox-img');
  if (!lightbox || !lightboxImg) return;

  function open(src, alt) {
    lightboxImg.src = src;
    lightboxImg.alt = alt || '';
    lightbox.hidden = false;
  }
  function close() {
    lightbox.hidden = true;
    lightboxImg.src = '';
  }

  document.addEventListener('click', (e) => {
    const img = e.target.closest('.detail-images img');
    if (img) {
      open(img.src, img.alt);
      return;
    }
    if (e.target === lightbox || e.target.closest('.lightbox-close')) {
      close();
    }
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') close();
  });
}

document.addEventListener('DOMContentLoaded', initLightbox);

// Light/dark theme toggle. The active theme is applied early (see the inline
// script in <head>) to avoid a flash of the wrong theme on page load; this
// just wires up the button and persists the choice.
function initThemeToggle() {
  const btn = document.getElementById('theme-toggle');
  if (!btn) return;
  const icon = btn.querySelector('.theme-toggle-icon');

  function setIcon(theme) {
    icon.textContent = theme === 'dark' ? '🌙' : '☀️';
  }

  setIcon(document.documentElement.getAttribute('data-theme') || 'light');

  btn.addEventListener('click', () => {
    const next = document.documentElement.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    localStorage.setItem('theme', next);
    setIcon(next);
  });
}

document.addEventListener('DOMContentLoaded', initThemeToggle);
