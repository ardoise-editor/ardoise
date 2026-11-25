(function () {
  var root = document.documentElement;
  var store = {
    get: function (k) { try { return localStorage.getItem(k); } catch (e) { return null; } },
    set: function (k, v) { try { localStorage.setItem(k, v); } catch (e) {} }
  };

  /* Language */
  function setLang(l) {
    root.lang = l;
    store.set('ardoise-lang', l);
    document.querySelectorAll('[data-set-lang]').forEach(function (b) {
      b.setAttribute('aria-pressed', b.dataset.setLang === l ? 'true' : 'false');
    });
    var t = root.getAttribute('data-title-' + l);
    if (t) document.title = t;
  }
  window.setLang = setLang;

  /* Theme */
  function applyTheme(t) {
    if (t === 'light' || t === 'dark') root.setAttribute('data-theme', t); else root.removeAttribute('data-theme');
  }
  function currentTheme() {
    return root.getAttribute('data-theme') ||
      (window.matchMedia && matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark');
  }

  document.addEventListener('DOMContentLoaded', function () {
    setLang(root.lang || 'fr');

    var tb = document.getElementById('theme-btn');
    if (tb) tb.addEventListener('click', function () {
      var n = currentTheme() === 'dark' ? 'light' : 'dark';
      applyTheme(n); store.set('ardoise-theme', n);
    });

    var bg = document.getElementById('burger'), nav = document.getElementById('nav');
    if (bg && nav) bg.addEventListener('click', function () {
      var o = nav.classList.toggle('open'); bg.setAttribute('aria-expanded', o ? 'true' : 'false');
    });

    /* Copy buttons */
    document.querySelectorAll('.copy').forEach(function (b) {
      b.addEventListener('click', function () {
        var src = b.closest('.cmd,.codewrap');
        var code = src && src.querySelector('code');
        if (!code) return;
        var txt = code.innerText.replace(/^\$ /gm, '');
        var done = function () {
          var o = b.innerHTML; b.classList.add('done');
          b.innerHTML = root.lang === 'fr' ? 'Copié' : 'Copied';
          setTimeout(function () { b.classList.remove('done'); b.innerHTML = o; }, 1600);
        };
        if (navigator.clipboard) navigator.clipboard.writeText(txt).then(done); else done();
      });
    });

    /* Tabs */
    document.querySelectorAll('[data-tabs]').forEach(function (g) {
      var btns = g.querySelectorAll('[role=tab]');
      function show(id) {
        btns.forEach(function (b) { b.setAttribute('aria-selected', b.dataset.tab === id ? 'true' : 'false'); });
        g.querySelectorAll('.tabpanel').forEach(function (p) { p.hidden = p.id !== id; });
      }
      btns.forEach(function (b) { b.addEventListener('click', function () { show(b.dataset.tab); }); });
      var ua = navigator.userAgent, pick = g.dataset.default;
      if (g.dataset.detect) {
        if (/Mac/.test(ua)) pick = g.dataset.mac || pick;
        else if (/Linux|X11/.test(ua)) pick = g.dataset.linux || pick;
      }
      if (location.hash && g.querySelector(location.hash + '.tabpanel')) pick = location.hash.slice(1);
      show(pick);
    });

    /* Missing media -> placeholder */
    function miss(el) { var s = el.closest('.slot'); if (s) s.classList.add('missing-' + (el.dataset.lang || 'fr')); }
    document.querySelectorAll('.slot img').forEach(function (i) {
      if (i.complete && i.naturalWidth === 0) miss(i);
      i.addEventListener('error', function () { miss(i); });
    });
    document.querySelectorAll('.slot video').forEach(function (v) {
      var s = v.querySelector('source');
      if (s) s.addEventListener('error', function () { miss(v); });
      v.addEventListener('error', function () { miss(v); });
      if (v.readyState === 0 && v.networkState === 3) miss(v);
      /* networkState 3 also means "still choosing a source": a video that then loads is not missing */
      v.addEventListener('loadedmetadata', function () {
        var s = v.closest('.slot'); if (s) s.classList.remove('missing-' + (v.dataset.lang || 'fr'));
      });
    });
    /* Hero: real screenshot or the built-in mock */
    document.querySelectorAll('img[data-fallback]').forEach(function (i) {
      var fb = document.querySelector(i.dataset.fallback);
      function use() { var b = i.closest('.win-body'); if (b) b.classList.add('nohero-' + (i.dataset.lang || 'fr')); }
      if (i.complete && i.naturalWidth === 0) use();
      i.addEventListener('error', use);
    });

    /* Click a screenshot or a video to see it large; click again for its real size */
    var box = document.createElement('dialog');
    if (box.showModal) {
      box.className = 'zoom';
      box.innerHTML = '<button class="zoom-close" type="button">\u00d7</button><div class="zoom-body"></div>';
      document.body.appendChild(box);
      var body = box.querySelector('.zoom-body'), closeBtn = box.querySelector('.zoom-close');
      var open = function (frame) {
        var lang = root.lang;
        var src = frame.querySelector(':scope > img[data-lang="' + lang + '"], :scope > video[data-lang="' + lang + '"]');
        if (!src || getComputedStyle(src).display === 'none') return;
        var big;
        if (src.tagName === 'VIDEO') {
          big = src.cloneNode(true);
          big.controls = true;
          big.muted = true;
        } else {
          big = document.createElement('img');
          big.src = src.currentSrc || src.src;
          big.alt = src.alt;
          big.addEventListener('click', function () { box.classList.toggle('real'); });
        }
        big.removeAttribute('data-lang');
        body.replaceChildren(big);
        closeBtn.setAttribute('aria-label', lang === 'fr' ? 'Fermer' : 'Close');
        box.classList.remove('real');
        box.showModal();
      };
      closeBtn.addEventListener('click', function () { box.close(); });
      box.addEventListener('click', function (e) { if (e.target === box || e.target === body) box.close(); });
      box.addEventListener('close', function () { body.replaceChildren(); });
      var frames = document.querySelectorAll('.slot .win-body, .hero-media .win-body');
      var label = function () {
        frames.forEach(function (f) { f.setAttribute('aria-label', root.lang === 'fr' ? 'Agrandir' : 'Enlarge'); });
      };
      frames.forEach(function (f) {
        f.classList.add('zoomable');
        f.tabIndex = 0;
        f.setAttribute('role', 'button');
        f.addEventListener('click', function () { open(f); });
        f.addEventListener('keydown', function (e) {
          if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); open(f); }
        });
      });
      label();
      new MutationObserver(label).observe(root, { attributes: true, attributeFilter: ['lang'] });
    }

    /* Reduced motion: don't autoplay videos */
    if (window.matchMedia && matchMedia('(prefers-reduced-motion: reduce)').matches) {
      document.querySelectorAll('video[autoplay]').forEach(function (v) { v.removeAttribute('autoplay'); v.controls = true; });
    }

    /* TOC scroll spy */
    var toc = document.querySelectorAll('.toc a[href^="#"]');
    if (toc.length && 'IntersectionObserver' in window) {
      var map = {};
      toc.forEach(function (a) { map[a.getAttribute('href').slice(1)] = a; });
      var io = new IntersectionObserver(function (es) {
        es.forEach(function (e) {
          if (e.isIntersecting) {
            toc.forEach(function (a) { a.classList.remove('on'); });
            var a = map[e.target.id]; if (a) a.classList.add('on');
          }
        });
      }, { rootMargin: '-90px 0px -70% 0px' });
      Object.keys(map).forEach(function (id) { var el = document.getElementById(id); if (el) io.observe(el); });
    }
  });
})();
