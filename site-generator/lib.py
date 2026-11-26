# -*- coding: utf-8 -*-
"""Tiny static-site toolkit for the Ardoise website (no dependency)."""
import html as _h
import os
import re

SITE = "https://ardoise-editor.github.io/ardoise/"
VERSION = ""  # set by build.py from Cargo.toml, before the pages are imported
REPO = "https://github.com/ardoise-editor/ardoise"
ORG = "https://github.com/ardoise-editor"

PAGES = []   # (filename, title_fr) for the sitemap
MEDIA = []   # registered media slots -> media/CAPTURES.md


def cargo_version(repo):
    """The app version: the one a release tag must match."""
    with open(os.path.join(repo, "Cargo.toml"), encoding="utf-8") as f:
        return re.search(r'^version\s*=\s*"([^"]+)"', f.read(), re.M).group(1)


def esc(s):
    return _h.escape(s, quote=True)


def L(fr, en):
    """Inline bilingual text."""
    return f'<span data-lang="fr">{fr}</span><span data-lang="en">{en}</span>'


def P(fr, en, cls=""):
    c = f' class="{cls}"' if cls else ""
    return f'<p{c} data-lang="fr">{fr}</p><p{c} data-lang="en">{en}</p>'


def H(level, fr, en, id=None):
    i = f' id="{id}"' if id else ""
    return f"<h{level}{i}>{L(fr, en)}</h{level}>"


def UL(items, tag="ul", cls=""):
    c = f' class="{cls}"' if cls else ""
    return f"<{tag}{c}>" + "".join(f"<li>{L(*it)}</li>" for it in items) + f"</{tag}>"


def callout(fr, en, label=("À savoir", "Good to know"), warn=False):
    c = "callout warn" if warn else "callout"
    return f'<div class="{c}"><b>{L(*label)}</b> — {L(fr, en)}</div>'


def kbd(*keys):
    return " ".join(f"<kbd>{esc(k)}</kbd>" for k in keys)


# ---------------------------------------------------------------- icons
def icon(name, size=22):
    paths = {
        "git": '<circle cx="6" cy="6" r="2.4"/><circle cx="6" cy="18" r="2.4"/><circle cx="18" cy="9" r="2.4"/><path d="M6 8.4v7.2M18 11.4c0 3.6-3 4-8.6 4.4"/>',
        "merge": '<circle cx="7" cy="5.5" r="2.4"/><circle cx="7" cy="18.5" r="2.4"/><circle cx="17" cy="18.5" r="2.4"/><path d="M7 7.9v8.2M17 16.1V11c0-3-2-5-5.5-5M13.5 3.5L11 6l2.5 2.5"/>',
        "database": '<ellipse cx="12" cy="5.5" rx="7" ry="2.8"/><path d="M5 5.5v13c0 1.5 3.1 2.8 7 2.8s7-1.3 7-2.8v-13M5 12c0 1.5 3.1 2.8 7 2.8s7-1.3 7-2.8"/>',
        "api": '<path d="M4 12h5l2-6 2 12 2-6h5"/>',
        "lint": '<path d="M12 3l8 3v6c0 4.5-3.2 7.6-8 9-4.8-1.4-8-4.5-8-9V6z"/><path d="M8.5 12l2.5 2.5 4.5-5"/>',
        "spell": '<path d="M4 17l4-11 4 11M5.5 13h5"/><path d="M14 15l2.5 2.5L21 11"/>',
        "history": '<path d="M4 12a8 8 0 1 0 2.5-5.8L4 8.5"/><path d="M4 4v4.5h4.5M12 8v4.5l3 2"/>',
        "tasks": '<path d="M9 6h11M9 12h11M9 18h11"/><path d="M3.5 6l1.3 1.3L7 5M3.5 12l1.3 1.3L7 11M3.5 18l1.3 1.3L7 17"/>',
        "remote": '<rect x="3" y="4" width="18" height="6" rx="1.5"/><rect x="3" y="14" width="18" height="6" rx="1.5"/><path d="M7 7h.01M7 17h.01"/>',
        "terminal": '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9.5l3 2.5-3 2.5M12.5 15H17"/>',
        "plug": '<path d="M9 3v5M15 3v5M6 8h12v3a6 6 0 0 1-12 0zM12 17v4"/>',
        "image": '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="1.8"/><path d="M21 16l-5-5-8 8"/>',
        "video": '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M10 9.5v5l4.5-2.5z"/>',
        "sun": '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.5 1.5M17.5 17.5L19 19M5 19l1.5-1.5M17.5 6.5L19 5"/>',
        "github": '<path d="M12 .5a11.5 11.5 0 0 0-3.64 22.41c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.37-3.88-1.37-.52-1.33-1.28-1.69-1.28-1.69-1.04-.71.08-.7.08-.7 1.15.08 1.76 1.19 1.76 1.19 1.03 1.76 2.69 1.25 3.35.96.1-.75.4-1.25.73-1.54-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.28 1.19-3.09-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.77 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.83 1.19 3.09 0 4.42-2.69 5.39-5.25 5.68.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5z" fill="currentColor" stroke="none"/>',
        "menu": '<path d="M4 7h16M4 12h16M4 17h16"/>',
    }
    return (f'<svg viewBox="0 0 24 24" width="{size}" height="{size}" fill="none" stroke="currentColor" '
            f'stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{paths[name]}</svg>')


# ---------------------------------------------------------------- media slots
def slot(name, fr, en, kind="img", ratio="16/10", title="Ardoise", caption=None, page="", size=None):
    """A screenshot / video slot. Shows media/<name> if present, else a placeholder
    that says what to capture. Every slot is listed in media/CAPTURES.md."""
    import re as _re
    fr, en = _re.sub(r"<[^>]+>", "", fr), _re.sub(r"<[^>]+>", "", en)
    MEDIA.append(dict(name=name, kind=kind, fr=fr, en=en, page=page, ratio=ratio, size=size))
    src = f"media/{name}"
    base, ext = name.rsplit(".", 1)
    media = ""
    # one file per language: <name> (French) and <base>-en.<ext> (English)
    for lg, nm in (("fr", name), ("en", f"{base}-en.{ext}")):
        s = f"media/{nm}"
        if kind == "video":
            poster = s.rsplit(".", 1)[0] + ".jpg"
            media += (f'<video data-lang="{lg}" muted loop playsinline autoplay preload="metadata" poster="{poster}">'
                      f'<source src="{s}" type="video/mp4"></video>')
        else:
            media += f'<img data-lang="{lg}" src="{s}" alt="{esc(en if lg == "en" else fr)}" loading="lazy" decoding="async">'
    if kind == "video":
        media += '<span class="vid-badge">VIDEO</span>'
        ico, lab = icon("video", 34), ("Vidéo à venir", "Video to come")
    else:
        ico, lab = icon("image", 34), ("Capture à venir", "Screenshot to come")
    r = ratio
    ph = (f'<div class="ph">{ico}<b>{src}</b>'
          f'<p>{L(esc(fr), esc(en))}</p><small>{L(*lab)}</small></div>')
    cap = f"<figcaption>{L(*caption)}</figcaption>" if caption else ""
    return (f'<figure class="slot" style="--ratio:{r}"><div class="win"><div class="win-bar"><i></i><i></i><i></i>'
            f'<span>{esc(title)}</span></div><div class="win-body">{media}{ph}</div></div>{cap}</figure>')


# ---------------------------------------------------------------- code blocks
def cmd(command, prompt="$"):
    return (f'<div class="cmd"><code><span class="p">{prompt}</span> {esc(command)}</code>'
            f'<button class="copy" type="button">{L("Copier", "Copy")}</button></div>')


def code(text, lang=None):
    lines = []
    for ln in text.strip("\n").split("\n"):
        if ln.startswith("# "):
            lines.append(f'<span class="c">{esc(ln)}</span>')
        elif ln.startswith("$ "):
            lines.append(f'<span class="p">$</span> {esc(ln[2:])}')
        else:
            lines.append(esc(ln))
    return (f'<div class="codewrap"><pre class="code"><code>{chr(10).join(lines)}</code></pre>'
            f'<button class="copy" type="button">{L("Copier", "Copy")}</button></div>')


def table(head, rows):
    h = "".join(f"<th>{L(*c) if isinstance(c, tuple) else c}</th>" for c in head)
    b = ""
    for r in rows:
        b += "<tr>" + "".join(f"<td>{L(*c) if isinstance(c, tuple) else c}</td>" for c in r) + "</tr>"
    return f"<table><thead><tr>{h}</tr></thead><tbody>{b}</tbody></table>"


# ---------------------------------------------------------------- layout
NAV = [
    ("home", "index.html", "Accueil", "Home"),
    ("guide", "guide.html", "Guide", "Guide"),
    ("plugins", "extensions.html", "Plugins", "Plugins"),
    ("install", "install.html", "Installation", "Install"),
]


def header(section):
    items = ""
    for key, href, fr, en in NAV:
        cur = ' aria-current="page"' if key == section else ""
        items += f'<a href="{href}"{cur}>{L(fr, en)}</a>'
    return f'''<a class="skip" href="#main">{L("Aller au contenu", "Skip to content")}</a>
<header class="site-header"><div class="wrap">
  <a class="brand" href="index.html"><img src="assets/ardoise.svg" alt="" width="28" height="28">Ardoise</a>
  <nav class="nav" id="nav" aria-label="Navigation">{items}</nav>
  <div class="spacer"></div>
  <div class="tools">
    <div class="seg" role="group" aria-label="Language"><button type="button" data-set-lang="fr" onclick="setLang('fr')">FR</button><button type="button" data-set-lang="en" onclick="setLang('en')">EN</button></div>
    <button class="icon-btn" id="theme-btn" type="button" aria-label="Theme / Thème">{icon("sun", 17)}</button>
    <a class="gh" href="{REPO}">{icon("github", 17)}<span>GitHub</span></a>
    <button class="icon-btn burger" id="burger" type="button" aria-label="Menu" aria-expanded="false">{icon("menu", 18)}</button>
  </div>
</div></header>'''


def footer():
    return f'''<footer class="site-footer"><div class="wrap">
 <div class="foot">
  <div><a class="brand" href="index.html"><img src="assets/ardoise.svg" alt="" width="28" height="28">Ardoise</a>
   <p>{L("Un éditeur de texte rapide pour macOS et Linux, écrit en Rust. Libre, licence MIT.", "A fast text editor for macOS and Linux, written in Rust. Free, MIT licensed.")}</p></div>
  <div><h6>{L("Application", "App")}</h6>
   <a href="index.html">{L("Accueil", "Home")}</a><a href="install.html">{L("Installation", "Install")}</a>
   <a href="guide.html">{L("Guide", "Guide")}</a><a href="{REPO}/blob/main/CHANGELOG.md">Changelog</a></div>
  <div><h6>Plugins</h6><a href="extensions.html">{L("Plugins officiels", "Official plugins")}</a>
   <a href="plugins.html">{L("Créer un plugin", "Write a plugin")}</a>
   <a href="{ORG}/plugin-sdk">SDK</a><a href="{ORG}/plugin-template">{L("Modèle", "Template")}</a></div>
  <div><h6>{L("Projet", "Project")}</h6><a href="{REPO}">GitHub</a><a href="{ORG}">{L("Organisation", "Organization")}</a>
   <a href="{REPO}/issues">{L("Signaler un problème", "Report an issue")}</a><a href="{REPO}/blob/main/LICENSE">{L("Licence MIT", "MIT license")}</a></div>
 </div>
 <div class="legal"><span>Ardoise Editor v{VERSION}</span><span>MIT · Rust · iced</span></div>
</div></footer>'''


def page(filename, section, title, desc, body, outdir, head_extra=""):
    """title/desc are (fr, en) tuples."""
    PAGES.append((filename, title[0]))
    t_fr, t_en = f"{title[0]} — Ardoise Editor", f"{title[1]} — Ardoise Editor"
    if filename == "index.html":
        t_fr, t_en = title[0], title[1]
    doc = f'''<!DOCTYPE html>
<html lang="fr" data-title-fr="{esc(t_fr)}" data-title-en="{esc(t_en)}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{esc(t_fr)}</title>
<meta name="description" content="{esc(desc[0])}">
<meta property="og:title" content="{esc(t_fr)}"><meta property="og:description" content="{esc(desc[0])}">
<meta property="og:type" content="website"><meta property="og:url" content="{SITE}{filename if filename != 'index.html' else ''}">
<meta name="color-scheme" content="dark light">
<link rel="icon" type="image/svg+xml" href="assets/ardoise.svg">
<link rel="stylesheet" href="assets/site.css">
<script>(function(){{try{{var d=document.documentElement,l=localStorage.getItem('ardoise-lang'),t=localStorage.getItem('ardoise-theme');
if(!l)l=(navigator.language||'fr').slice(0,2)==='fr'?'fr':'en';d.lang=l;if(t)d.setAttribute('data-theme',t);}}catch(e){{}}}})();</script>
<script defer src="assets/site.js"></script>
{head_extra}
</head>
<body>
{header(section)}
<main id="main">
{body}
</main>
{footer()}
</body>
</html>
'''
    with open(os.path.join(outdir, filename), "w", encoding="utf-8") as f:
        f.write(doc)


def cta_block(fr_title, en_title, fr_text, en_text, buttons):
    btns = "".join(
        f'<a class="btn{" primary" if p else ""}" href="{href}">{L(fr, en)}</a>' for href, fr, en, p in buttons)
    return (f'<section class="block tight"><div class="wrap"><div class="cta"><h2>{L(fr_title, en_title)}</h2>'
            f'<p class="lead">{L(fr_text, en_text)}</p><div class="cta-row">{btns}</div></div></div></section>')


def doc_layout(toc_groups, content, pager=None, active=None):
    """toc_groups: [(heading_fr, heading_en, [(href, fr, en)])]"""
    toc = ""
    for hf, he, items in toc_groups:
        if hf:
            toc += f"<h6>{L(hf, he)}</h6>"
        for href, fr, en in items:
            cur = ' aria-current="page"' if href == active else ""
            toc += f'<a href="{href}"{cur}>{L(fr, en)}</a>'
    pg = ""
    if pager:
        prev, nxt = pager
        if prev:
            pg += f'<a href="{prev[0]}"><small>{L("← Précédent", "← Previous")}</small>{L(prev[1], prev[2])}</a>'
        if nxt:
            pg += f'<a class="next" href="{nxt[0]}"><small>{L("Suivant →", "Next →")}</small>{L(nxt[1], nxt[2])}</a>'
        pg = f'<div class="pager">{pg}</div>'
    return (f'<div class="wrap doc"><nav class="toc" aria-label="Contents">{toc}</nav>'
            f'<div class="prose">{content}{pg}</div></div>')


def write_captures(outdir):
    os.makedirs(os.path.join(outdir, "media"), exist_ok=True)
    seen = {}
    for m in MEDIA:
        seen.setdefault(m["name"], m)
    P1 = {"hero.png", "goto-definition.mp4", "tour.mp4", "welcome.png", "search-everywhere.png", "editing.png", "workspace.png",
          "theme-dark.png", "theme-light.png", "plugins-preferences.png", "plugin-git-commit.png", "plugin-git-log.png",
          "plugin-database-tree.png", "plugin-database-console.png", "plugin-terminal-panel.png", "plugin-api-request.png",
          "plugin-git-demo.mp4", "plugin-database-demo.mp4", "welcome-first-launch.mp4", "plugins-install.mp4"}
    for m in seen.values():
        m["prio"] = "1" if m["name"] in P1 else ("2" if m["page"].startswith("plugin-") else "3")
    imgs = sorted([m for m in seen.values() if m["kind"] == "img"], key=lambda m: m["prio"])
    vids = sorted([m for m in seen.values() if m["kind"] == "video"], key=lambda m: m["prio"])
    out = ["# Médias du site Ardoise — liste des captures à faire\n",
           "Déposez chaque fichier dans ce dossier (`docs/media/`) **avec exactement le nom indiqué** : "
           "le site l'affiche automatiquement à la place du cadre « à venir ». **Chaque média existe en deux langues** : `nom.ext` = interface en français, "
           "`nom-en.ext` = interface en anglais (ex. `hero.png` et `hero-en.png`, `tour.mp4` et `tour-en.mp4`, poster `tour-en.jpg`).\n",
           "## Par où commencer\n",
           "**Prio 1** (20 fichiers) = l'essentiel : le site est déjà présentable avec eux seuls. **Prio 2** = pages des plugins. **Prio 3** = pages du guide. "
           "Tant qu'un fichier manque, le site affiche un cadre qui dit quoi capturer : rien ne casse.\n",
           "## Conseils\n",
           "- **Captures** : PNG, fenêtre d'Ardoise seule (sans la barre de titre du système), thème **sombre** par défaut "
           "(sauf mention contraire), police de texte lisible, un projet réaliste ouvert (pas de données personnelles). "
           "Largeur recommandée 1600 px ; ratio indiqué pour chaque capture (le cadre recadre en haut à gauche).",
           "- **Vidéos** : MP4 H.264, **sans son**, 6 à 15 secondes, en boucle, 1280×800 environ. "
           "Une image d'aperçu optionnelle du même nom en `.jpg` est utilisée comme poster.",
           "- Conversion : `ffmpeg -i capture.mkv -an -vf scale=1280:-2 -c:v libx264 -crf 26 -pix_fmt yuv420p -movflags +faststart nom.mp4`",
           "- Enregistrement Linux : OBS Studio ou Kooha (Wayland) ; capture : outil de capture de GNOME (`Maj+Impr`) ou Flameshot.",
           "- Poster : `ffmpeg -i nom.mp4 -frames:v 1 nom.jpg`\n",
           f"## Vidéos ({len(vids)})\n", "| Prio | Fichier | À montrer | Page |", "|---|---|---|---|"]
    for m in vids:
        out.append(f"| {m['prio']} | `{m['name']}` | {m['fr']} | {m['page']} |")
    out += ["", f"## Captures d'écran ({len(imgs)})\n", "| Prio | Fichier | À montrer | Ratio | Page |", "|---|---|---|---|---|"]
    for m in imgs:
        out.append(f"| {m['prio']} | `{m['name']}` | {m['fr']} | {m['ratio']} | {m['page']} |")
    with open(os.path.join(outdir, "media", "CAPTURES.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(out) + "\n")
    return len(imgs), len(vids)


def write_brief(outdir):
    seen = {}
    for m in MEDIA:
        seen.setdefault(m["name"], m)
    P1 = {"hero.png", "goto-definition.mp4", "tour.mp4", "welcome.png", "search-everywhere.png", "editing.png", "workspace.png",
          "theme-dark.png", "theme-light.png", "plugins-preferences.png", "plugin-git-commit.png", "plugin-git-log.png",
          "plugin-database-tree.png", "plugin-database-console.png", "plugin-terminal-panel.png", "plugin-api-request.png",
          "plugin-git-demo.mp4", "plugin-database-demo.mp4", "welcome-first-launch.mp4", "plugins-install.mp4"}
    rows = sorted(seen.values(), key=lambda m: ("1" if m["name"] in P1 else ("2" if m["page"].startswith("plugin-") else "3"), m["name"]))
    t = ["""# Brief pour Claude Code — captures et vidéos du site Ardoise

## Objectif
Produire les captures d'écran (PNG) et courtes vidéos (MP4) listées plus bas, avec **exactement** les noms de fichier indiqués,
et les déposer dans `docs/media/` du dépôt `ardoise-editor/ardoise`. Le site les affiche automatiquement : rien d'autre à modifier.
Ne modifie ni les pages HTML ni `docs/plugins.html`.

## Méthode
1. **Environnement** : `echo $XDG_SESSION_TYPE`. Sous X11 : `xdotool` (clavier/souris), `ffmpeg -f x11grab` (vidéo), `import`/`scrot`/`gnome-screenshot -w` (capture).
   Sous Wayland : `gnome-screenshot`, `grim`/`wf-recorder` ou Kooha ; piloter la fenêtre à la main ou via `ydotool`. Installe ce qui manque (`sudo apt install ...`) en me le disant.
   Alternative propre : lancer une session X11 isolée (`Xvfb :99 -screen 0 1600x1000x24` + un gestionnaire de fenêtres léger) et y lancer `ardoise` : fenêtre de taille fixe, rien de personnel à l'écran.
2. **Application** : Ardoise est installé (`ardoise`). Fenêtre ≈ 1600×1000, thème **sombre**, langue **français** (puis une seconde série en **anglais**, fichiers `nom-en.ext`), police lisible. Capturer la **fenêtre seule**.
3. **Données de démo** : jamais de fichiers personnels. Utiliser un clone du dépôt `ardoise-editor/ardoise` (code Rust, README) comme projet ouvert ; créer un projet Git jetable dans `/tmp/demo` pour les démos Git.
   Pour la base de données (Docker disponible) : `docker run -d --name ardoise-pg -e POSTGRES_PASSWORD=demo -p 5432:5432 postgres:16`, charger un petit schéma d'exemple (clients, commandes, produits avec clés étrangères) ; idem MySQL si besoin. Supprimer les conteneurs à la fin.
   Pour l'API : un serveur local simple (ex. `python3 -m http.server`, ou `docker run -p 8080:80 kennethreitz/httpbin`).
4. **Captures** : PNG, ~1600 px de large, nom exact, ratio indiqué (le cadre du site recadre en haut à gauche). Sans menu ni notification système.
5. **Vidéos** : 6 à 15 s, 1280×800 environ, **sans son**, gestes lents et lisibles, début et fin propres (le fichier boucle).
   Conversion : `ffmpeg -i brut.mkv -an -vf scale=1280:-2 -c:v libx264 -crf 26 -pix_fmt yuv420p -movflags +faststart NOM.mp4` puis poster : `ffmpeg -i NOM.mp4 -frames:v 1 NOM.jpg`.
6. **Ordre** : tout ce qui est « Prio 1 » d'abord, puis Prio 2 (pages plugins), puis Prio 3 (guide). Après chaque lot, ouvrir le site en local
   (`cd docs && python3 -m http.server 8000`) et vérifier que le cadre « à venir » a bien été remplacé par le média.
7. Ne pas committer sans me demander. Dire à la fin ce qui n'a pas pu être fait.

## Liste des médias

| Prio | Fichier | Type | À montrer | Ratio | Page |
|---|---|---|---|---|---|"""]
    for m in rows:
        pr = "1" if m["name"] in P1 else ("2" if m["page"].startswith("plugin-") else "3")
        t.append(f"| {pr} | `{m['name']}` | {'vidéo' if m['kind'] == 'video' else 'capture'} | {m['fr']} | {m['ratio']} | {m['page']} |")
    with open(os.path.join(outdir, "media", "BRIEF-CLAUDE-CODE.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(t) + "\n")
