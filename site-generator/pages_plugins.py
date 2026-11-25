# -*- coding: utf-8 -*-
from lib import *
from plugins_data import PLUGINS, BY_SLUG


def catalog(out):
    cards = ""
    for p in PLUGINS:
        cards += (f'<a class="card" href="plugin-{p["slug"]}.html"><div class="pi">{icon(p["icon"])}</div>'
                  f'<h3>{L(*p["name"])} <span class="ver">v{p["ver"]}</span></h3><p>{L(*p["tag"])}</p>'
                  f'<span class="more">{L("Voir le détail", "See details")} →</span></a>')
    steps = f'''<ol class="steps">
<li><h4>{L("Ouvrez les Préférences", "Open the Preferences")}</h4><p>{L("Menu <b>Fichier › Préférences…</b>, puis la page <b>Plugins</b>.", "Menu <b>File › Preferences…</b>, then the <b>Plugins</b> page.")}</p></li>
<li><h4>{L("Cherchez et installez", "Search and install")}</h4><p>{L("Un champ de recherche filtre la liste. Cliquez sur <b>Installer</b> : le plugin est téléchargé depuis son dépôt, sans droits administrateur. Les plugins installés à part de la liste officielle apparaissent aussi.", "A search box filters the list. Click <b>Install</b>: the plugin is downloaded from its repository, with no administrator rights. Plugins installed apart from the official list show too.")}</p></li>
<li><h4>{L("Utilisez son panneau", "Use its panel")}</h4><p>{L("Le panneau du plugin est listé dans <b>Affichage › Panneau</b>, et ses actions dans la recherche d'actions et les menus. Pour le retirer : <b>Désinstaller</b>, sur la même page.", "The plugin's panel is listed in <b>View › Panel</b>, and its actions in the search for actions and the menus. To remove it: <b>Uninstall</b>, on the same page.")}</p></li></ol>'''
    body = f'''<section class="page-head"><div class="wrap">
<div class="crumbs"><a href="index.html">{L("Accueil", "Home")}</a> / Plugins</div>
<h1>{L("Plugins officiels", "Official plugins")}</h1>
<p class="lead">{L("Ardoise est un éditeur simple à l'installation : les plugins ajoutent le reste. Chaque plugin officiel vit dans son propre dépôt de l'organisation ardoise-editor et est signé avec la clé des versions d'Ardoise.", "Ardoise is a plain editor out of the box: plugins add the rest. Each official plugin lives in its own repository of the ardoise-editor organization and is signed with Ardoise's release key.")}</p></div></section>
<section class="block tight" style="padding-top:24px"><div class="wrap"><div class="grid g3">{cards}</div></div></section>
<section class="block" style="background:var(--side);border-block:1px solid var(--line)"><div class="wrap split eq" id="how">
<div class="prose" style="max-width:none"><span class="eyebrow">{L("Installation", "Install")}</span>
<h2 style="margin-top:12px">{L("Un plugin, trois gestes", "A plugin in three moves")}</h2>{steps}
{callout("à la première ouverture, une fenêtre d'accueil propose ces plugins en un clic ; les outils que vous utilisiez avant l'arrivée des plugins s'installent d'eux-mêmes.", "on the first launch a welcome window offers these plugins in one click; the tools you used before plugins existed install themselves.")}</div>
{slot("plugins-preferences.png", "Préférences › Plugins : la liste des plugins officiels avec le champ de recherche, certains installés, d'autres non.", "Preferences › Plugins: the list of official plugins with the search box, some installed, some not.", page="extensions.html", title="Préférences › Plugins")}
</div></section>
<section class="block"><div class="wrap" style="max-width:900px"><div class="prose" style="max-width:none">
{H(2, "Écrire votre plugin", "Write your own plugin", "write")}
{P("Un plugin est un programme accompagné d'un fichier <code>plugin.json</code> : l'éditeur le lance et ils dialoguent en JSON-RPC sur l'entrée et la sortie standard. Les panneaux sont des arbres de nœuds (colonne, ligne, texte, bouton, champ, liste, arbre…) que l'éditeur dessine avec ses propres composants, dans son thème. Écrivez-le en Rust avec le SDK, ou dans n'importe quel langage qui sait lire et écrire sur ses flux standard.", "A plugin is a program with a <code>plugin.json</code> beside it: the editor starts it and they talk JSON-RPC over standard input and output. Panels are trees of nodes (column, row, text, button, field, list, tree…) that the editor draws with its own widgets, in its theme. Write it in Rust with the SDK, or in any language that reads and writes its standard streams.")}
<div class="cta-row"><a class="btn primary" href="plugins.html">{L("Lire le guide du développeur", "Read the developer guide")}</a>
<a class="btn" href="{ORG}/plugin-template">{L("Modèle de plugin", "Plugin template")}</a><a class="btn" href="{ORG}/plugin-sdk">SDK</a></div>
{P("Pour essayer un plugin en cours de développement : <b>Préférences › Plugins › Lier un dossier…</b>. Pour publier : poussez un tag <code>vX.Y.Z</code> ; le workflow du modèle construit les archives Linux et macOS et la somme de contrôle, et n'importe qui peut ensuite l'installer depuis Préférences › Plugins en saisissant <code>propriétaire/nom</code>.", "To try a plugin under development: <b>Preferences › Plugins › Link a folder…</b>. To publish: push a <code>vX.Y.Z</code> tag; the template's workflow builds the Linux and macOS archives and the checksums, and anyone can then install it from Preferences › Plugins by typing <code>owner/name</code>.")}
</div></div></section>'''
    page("extensions.html", "plugins", ("Plugins officiels", "Official plugins"),
         ("Les neuf plugins officiels d'Ardoise Editor : Git, base de données, API, lint, orthographe, historique local, tâches, fichiers distants et terminal.",
          "The nine official Ardoise Editor plugins: Git, database, API, lint, spelling, local history, tasks, remote files and terminal."),
         body, out)


def one(p, out, idx):
    slug = p["slug"]
    fn = f"plugin-{slug}.html"
    vids = [m for m in p["media"] if m[3] == "video"]
    imgs = [m for m in p["media"] if m[3] == "img"]

    def S(m):
        return slot(m[0], m[1], m[2], kind=m[3], ratio=m[4], page=fn, title=f'Ardoise — {p["name"][1]}')

    hero_media = S(vids[0]) if vids else (S(imgs[0]) if imgs else "")
    rest_imgs = imgs if vids else imgs[1:]

    toc = [("#about", "Présentation", "Overview")]
    sections = f'<h2 id="about">{L("Présentation", "Overview")}</h2>{P(*p["intro"])}'
    for gi, (gt, items) in enumerate(p["groups"]):
        gid = f"g{gi}"
        toc.append((f"#{gid}", gt[0], gt[1]))
        sections += f'<h2 id="{gid}">{L(*gt)}</h2>{UL(items)}'
        if gi < len(rest_imgs):
            sections += S(rest_imgs[gi])
    for m in rest_imgs[len(p["groups"]):]:
        sections += S(m)
    if p["keys"]:
        toc.append(("#keys", "Raccourcis", "Shortcuts"))
        rows = [[" ".join(f"<kbd>{k}</kbd>" for k in ks), (fr, en)] for ks, fr, en in p["keys"]]
        sections += (f'<h2 id="keys">{L("Raccourcis", "Shortcuts")}</h2>' + table([("Touches", "Keys"), ("Action", "Action")], rows) +
                     P("Tous les raccourcis se changent dans Préférences › Raccourcis (regroupés sous le nom du plugin).", "Every shortcut can be changed in Preferences › Shortcuts (grouped under the plugin's name)."))
    toc.append(("#install", "Installer", "Install"))
    sections += f'<h2 id="install">{L("Installer et régler", "Install and configure")}</h2>'
    sections += P(f'Ouvrez <b>Fichier › Préférences… › Plugins</b>, cherchez « {p["name"][0]} » et cliquez sur <b>Installer</b>.',
                  f'Open <b>File › Preferences… › Plugins</b>, search for “{p["name"][1]}” and click <b>Install</b>.')
    if p["needs"]:
        sections += f'<h3>{L("Prérequis", "Requirements")}</h3>{UL(p["needs"])}'
    sections += f'<h3>{L("Réglages", "Settings")}</h3>{P(*p["settings"])}'
    repo, ver = p["repo"], p["ver"]
    sections += f'<h3>{L("Code source et licence", "Source and license")}</h3>'
    sections += P(f'Dépôt : <a href="{ORG}/{repo}">ardoise-editor/{repo}</a> · identifiant <code>ardoise.{slug}</code> · version {ver} · licence MIT.',
                  f'Repository: <a href="{ORG}/{repo}">ardoise-editor/{repo}</a> · id <code>ardoise.{slug}</code> · version {ver} · MIT license.')

    prev = PLUGINS[idx - 1] if idx > 0 else None
    nxt = PLUGINS[idx + 1] if idx < len(PLUGINS) - 1 else None
    pager = (("plugin-%s.html" % prev["slug"], prev["name"][0], prev["name"][1]) if prev else ("extensions.html", "Tous les plugins", "All plugins"),
             ("plugin-%s.html" % nxt["slug"], nxt["name"][0], nxt["name"][1]) if nxt else ("extensions.html", "Tous les plugins", "All plugins"))
    others = [("plugin-%s.html" % q["slug"], q["name"][0], q["name"][1]) for q in PLUGINS]
    groups = [("Cette page", "On this page", [(h, f, e) for h, f, e in toc]), ("Plugins", "Plugins", others)]
    content = doc_layout(groups, sections, pager, active=fn)

    head = f'''<section class="page-head"><div class="wrap split">
<div><div class="crumbs"><a href="index.html">{L("Accueil", "Home")}</a> / <a href="extensions.html">Plugins</a> / {L(*p["name"])}</div>
<div class="pi" style="margin-top:22px;width:54px;height:54px">{icon(p["icon"], 28)}</div>
<h1>{L(*p["name"])}</h1><p class="lead">{L(*p["tag"])}</p>
<div class="cta-row"><a class="btn primary" href="install.html">{L("Installer Ardoise", "Install Ardoise")}</a>
<a class="btn" href="{ORG}/{p["repo"]}">{icon("github", 17)} GitHub</a></div>
<div class="badge-row"><span class="chip">v{p["ver"]}</span><span class="chip">ardoise.{slug}</span><span class="chip">MIT</span></div></div>
<div>{hero_media}</div></div></section>'''
    page(fn, "plugins", (f'Plugin {p["name"][0]}', f'{p["name"][1]} plugin'),
         (f'{p["tag"][0]} Plugin officiel d\'Ardoise Editor.', f'{p["tag"][1]} Official Ardoise Editor plugin.'),
         head + content, out)


def build(out):
    catalog(out)
    for i, p in enumerate(PLUGINS):
        one(p, out, i)
