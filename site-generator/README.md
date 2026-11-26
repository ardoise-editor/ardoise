# Site generator

Builds the website in `docs/` (Python 3.9+, no dependency). Edit the generator, not the HTML.

```sh
python3 site-generator/build.py --repo . --out docs
python3 -m unittest discover -s site-generator/tests   # generator and published pages
node site-generator/tests/browser.mjs                  # needs Node 22+ and Chrome
```

The version shown on the site is read from `Cargo.toml`. After changing it, rebuild and commit `docs/`:
the CI fails while `docs/` differs from what the generator makes.

The versions of the plugins are those of their latest releases: the site is rebuilt with them when
it is deployed, after each merge and once a day (to do it at once: Actions › GitHub Pages › Run workflow).
`plugin_versions.json` is what `docs/` in the repository shows; `python3 site-generator/plugin_versions.py`
brings it up to date, then rebuild.

Each screenshot or video exists in two languages: `name.png` (French) and `name-en.png` (English).
