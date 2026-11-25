# Site generator

Builds the website in `docs/` (Python 3.9+, no dependency). Edit the generator, not the HTML.

```sh
python3 site-generator/build.py --repo . --out docs
python3 -m unittest discover -s site-generator/tests   # generator and published pages
node site-generator/tests/browser.mjs                  # needs Node 22+ and Chrome
```

Each screenshot or video exists in two languages: `name.png` (French) and `name-en.png` (English).
