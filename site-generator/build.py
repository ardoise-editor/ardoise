# -*- coding: utf-8 -*-
"""Generates the Ardoise website (static HTML in docs/).

Usage:  python3 site-generator/build.py --repo /path/to/ardoise --out /path/to/ardoise/docs
Never deletes anything: it only (over)writes generated files, so media/ you added is kept.
"""
import argparse, os, shutil, sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
ap = argparse.ArgumentParser()
ap.add_argument("--repo", default=os.environ.get("ARDOISE_REPO", "."), help="clone of ardoise-editor/ardoise")
ap.add_argument("--out", default=None, help="output folder (default: <repo>/docs)")
a = ap.parse_args()
REPO = os.path.abspath(a.repo)
OUT = os.path.abspath(a.out or os.path.join(REPO, "docs"))
SRC = os.path.join(REPO, "docs")
import lib
os.makedirs(os.path.join(OUT, "assets"), exist_ok=True)
if os.path.abspath(SRC) != OUT:
    shutil.copytree(os.path.join(SRC, "fonts"), os.path.join(OUT, "fonts"), dirs_exist_ok=True)
for f in ("site.css", "site.js"):
    shutil.copy(os.path.join(HERE, "assets", f), os.path.join(OUT, "assets", f))
shutil.copy(os.path.join(REPO, "assets", "ardoise.svg"), os.path.join(OUT, "assets", "ardoise.svg"))
import pages_home, pages_install, pages_plugins
pages_home.build(OUT); pages_install.build(OUT); pages_plugins.build(OUT)
import pages_guide; pages_guide.build(OUT)

# plugins.html = the developer guide (the app links to it): kept, only its menu is extended
t = open(os.path.join(SRC, "plugins.html"), encoding="utf-8").read()
if 'href="extensions.html"' not in t:
    nav = ('<a class="link" href="guide.html">Guide</a>\n  <a class="link" href="extensions.html">Plugins</a>\n  '
           '<a class="link" href="install.html"><span data-lang="en">Install</span><span data-lang="fr">Installation</span></a>\n  ')
    t = t.replace('<a class="link" href="https://github.com/ardoise-editor/plugin-template">', nav + '<a class="link" href="https://github.com/ardoise-editor/plugin-template">', 1)
open(os.path.join(OUT, "plugins.html"), "w", encoding="utf-8").write(t)
lib.PAGES.append(("plugins.html", "Créer un plugin"))
open(os.path.join(OUT, ".nojekyll"), "w").close()
body = ('<section class="page-head"><div class="wrap"><h1>404</h1><p class="lead">' + lib.L("Cette page n'existe pas.", "This page does not exist.") +
        '</p><div class="cta-row"><a class="btn primary" href="index.html">' + lib.L("Retour à l'accueil", "Back home") + '</a></div></div></section>')
lib.page("404.html", "", ("Page introuvable", "Page not found"), ("Page introuvable", "Page not found"), body, OUT)
lib.PAGES.pop()
urls = "".join("<url><loc>%s%s</loc></url>" % (lib.SITE, "" if f == "index.html" else f) for f, _ in lib.PAGES)
open(os.path.join(OUT, "sitemap.xml"), "w").write('<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">' + urls + "</urlset>")
open(os.path.join(OUT, "robots.txt"), "w").write("User-agent: *\nAllow: /\nSitemap: " + lib.SITE + "sitemap.xml\n")
n = lib.write_captures(OUT)
lib.write_brief(OUT)
print(len(lib.PAGES), "pages;", n, "(images, videos) ->", OUT)
