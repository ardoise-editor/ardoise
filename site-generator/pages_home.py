# -*- coding: utf-8 -*-
from lib import *
from plugins_data import PLUGINS

ONELINER = "curl -fsSL https://raw.githubusercontent.com/ardoise-editor/ardoise/main/install.sh | bash"


def mock_editor():
    rows = [
        ('<span class="c">// Palette de couleurs</span>',),
        ('<span class="k">pub struct</span> <span class="t">Palette</span> {',),
        ('    <span class="k">pub</span> bg: <span class="t">Color</span>,',),
        ('    <span class="k">pub</span> accent: <span class="t">Color</span>,',),
        ('}',),
        ('',),
        ('<span class="k">pub fn</span> <span class="f">palette</span>(dark: <span class="t">bool</span>) -&gt; <span class="t">Palette</span> {',),
        ('    <span class="k">if</span> dark {',),
        ('        <span class="t">Palette</span> {',),
        ('            bg: <span class="f">rgb</span>(<span class="n">0x14</span>, <span class="n">0x15</span>, <span class="n">0x1A</span>),',),
        ('            accent: <span class="f">rgb</span>(<span class="n">0xFF</span>, <span class="n">0x9B</span>, <span class="n">0x6B</span>),',),
        ('        }',),
        ('    } <span class="k">else</span> {',),
        ('        <span class="t">Palette</span> { bg: <span class="f">rgb</span>(<span class="n">0xE9</span>, <span class="n">0xEC</span>, <span class="n">0xF3</span>), accent: <span class="f">rgb</span>(<span class="n">0xD2</span>, <span class="n">0x56</span>, <span class="n">0x1F</span>) }',),
        ('    }',),
        ('}',),
    ]
    lines = "".join(f"<div><i>{i + 1}</i><span>{r[0]}</span></div>" for i, r in enumerate(rows))
    return f'''<div class="mock" id="hero-mock" hidden aria-hidden="true">
<div class="mock-side"><h6>Projets</h6><p>▾ ardoise</p><p>&nbsp;&nbsp;▸ assets</p><p>&nbsp;&nbsp;▾ src</p><p>&nbsp;&nbsp;&nbsp;&nbsp;app.rs</p><p class="on">&nbsp;&nbsp;&nbsp;&nbsp;theme.rs</p><p>&nbsp;&nbsp;&nbsp;&nbsp;tabs.rs</p><p>&nbsp;&nbsp;Cargo.toml</p><p>&nbsp;&nbsp;README.md</p></div>
<div class="mock-main"><div class="mock-tabs"><span>app.rs</span><span class="on">theme.rs</span><span>Cargo.toml</span></div>
<div class="mock-code">{lines}</div>
<div class="mock-status"><span>Rust · UTF-8 · LF</span><span>Ln 10, Col 24</span></div></div></div>'''


def hero():
    img = "".join(f'<img data-lang="{lg}" src="media/{n}" alt="Ardoise Editor" data-fallback="#hero-mock" '
                  'style="aspect-ratio:16/10;object-fit:cover;object-position:top left;width:100%">'
                  for lg, n in (("fr", "hero.png"), ("en", "hero-en.png")))
    MEDIA.append(dict(name="hero.png", kind="img", page="index.html", ratio="16/10",
                      fr="La fenêtre d'Ardoise entière : barre latérale avec un projet, 3 onglets ouverts, un fichier Rust coloré, barre d'état. C'est la vitrine : soignez-la.",
                      en="The whole Ardoise window: sidebar with a project, 3 open tabs, a highlighted Rust file, status bar.", size=None))
    return f'''<section class="hero"><div class="wrap hero-grid">
<div>
 <span class="eyebrow">{L("Open source · MIT · macOS et Linux", "Open source · MIT · macOS & Linux")}</span>
 <h1>{L("L'éditeur de texte rapide, <em>qui s'équipe</em> à votre mesure.", "A fast text editor that <em>grows</em> to fit you.")}</h1>
 <p class="lead">{L("Ardoise est écrit en Rust : il démarre tout de suite, colore plus de 30 langages, trouve la définition d'une fonction d'un Ctrl+clic. Au départ c'est un éditeur simple ; Git, bases de données, client d'API ou terminal s'ajoutent en un clic, avec des plugins officiels signés.", "Ardoise is written in Rust: it starts at once, highlights 30+ languages and finds a definition with a Ctrl+click. It begins as a plain editor; Git, databases, an API client or a terminal install in one click, as signed official plugins.")}</p>
 <div class="cta-row"><a class="btn primary" href="install.html">{L("Installer Ardoise", "Install Ardoise")}</a><a class="btn" href="guide.html">{L("Lire le guide", "Read the guide")}</a></div>
 <div style="margin-top:24px;max-width:640px">{cmd(ONELINER)}</div>
 <div class="badge-row"><span class="chip">v{VERSION}</span><span class="chip">Rust + iced</span><span class="chip">macOS (Apple Silicon)</span><span class="chip">Linux x86_64</span><span class="chip">FR · EN</span></div>
</div>
<div class="hero-media"><div class="win"><div class="win-bar"><i></i><i></i><i></i><span>Ardoise — theme.rs</span></div>
<div class="win-body">{img}{mock_editor()}</div></div></div>
</div>
<div class="wrap"><div class="stats">
 <div><b>30+</b><span>{L("langages colorés (grammaires TextMate)", "languages highlighted (TextMate grammars)")}</span></div>
 <div><b>9</b><span>{L("plugins officiels, signés", "official plugins, signed")}</span></div>
 <div><b>64 Mo</b><span>{L("taille maximale d'un fichier ouvert", "largest file that opens")}</span></div>
 <div><b>MIT</b><span>{L("libre, sans filigrane ni compte", "free, no account, no watermark")}</span></div>
</div></div></section>'''


def pillars():
    cards = [
        ("01", ("Rapide et natif", "Fast and native"),
         ("Une application native en Rust, pas une page web emballée. Elle s'ouvre vite, reste réactive sur de gros fichiers et garde votre session d'une fois sur l'autre, même le texte non enregistré.", "A native Rust application, not a wrapped web page. It opens fast, stays responsive on large files and keeps your session between runs, unsaved text included.")),
        ("02", ("Simple au départ", "Simple to begin with"),
         ("Un éditeur sobre à l'installation. Ce dont vous avez besoin s'ajoute depuis Préférences › Plugins — ou dès le premier lancement, dans une fenêtre d'accueil.", "A plain editor out of the box. What you need is added from Preferences › Plugins — or on first launch, in a welcome window.")),
        ("03", ("Chez vous, tout simplement", "Yours, plainly"),
         ("Fichiers locaux, session stockée sur votre machine et lisible par vous seul, mots de passe des plugins dans le trousseau du système. Désinstallation complète en une commande.", "Local files, a session stored on your machine and readable by you only, plugin passwords in the system keychain. A complete uninstall in one command.")),
        ("04", ("Mises à jour vérifiées", "Verified updates"),
         ("Aide › Rechercher des mises à jour. Les versions sont signées (Ed25519) ; une mise à jour dont la signature ou la somme de contrôle ne correspond pas est refusée.", "Help › Check for updates. Releases are signed (Ed25519); an update whose signature or checksum does not match is refused.")),
    ]
    out = "".join(f'<div class="card"><span class="num">{n}</span><h3>{L(*t)}</h3><p>{L(*d)}</p></div>' for n, t, d in cards)
    return f'''<section class="block"><div class="wrap">
<div class="sec-head"><span class="eyebrow">{L("Pensé pour le travail de tous les jours", "Made for everyday work")}</span>
<h2>{L("Ce qui compte, rien de superflu.", "What matters, nothing superfluous.")}</h2></div>
<div class="grid g4">{out}</div></div></section>'''


def showcase():
    rows = []
    rows.append((False, "Navigation", ("Aller à la définition, sans configuration", "Go to definition, no setup"),
                 ("Ctrl+clic (Cmd+clic sur macOS) sur une fonction, une variable, une classe ou une interface : l'éditeur saute à sa définition. Sur la définition elle-même, il liste où elle est utilisée. Il ne répond que s'il est sûr.", "Ctrl+click (Cmd+click on macOS) on a function, variable, class or interface and the editor jumps to where it is defined. On the definition itself, it lists where it is used. It only answers when it is sure."),
                 [("Précédent / Suivant (Alt+←/→, boutons de la souris) à travers les sauts, onglets compris", "Back / Forward (Alt+←/→, mouse buttons) across your jumps, tabs included"),
                  ("Serveurs de langage facultatifs, installés sans droits administrateur", "Optional language servers, installed without administrator rights"),
                  ("Sans serveur : une recherche de motifs prudente", "Without one: a careful pattern search")],
                 ("guide-navigation.html", "Voir la navigation", "See navigation"),
                 slot("goto-definition.mp4", "Ctrl+clic sur un appel de fonction : le fichier de la définition s'ouvre à la bonne ligne ; puis Alt+← pour revenir. Sur un projet réel (le code d'Ardoise convient).", "Ctrl+click a function call: the definition file opens at the right line; then Alt+← to go back.", kind="video", page="index.html", title="Aller à la définition")))
    rows.append((True, "Édition", ("Coloration, pliage, recherche : l'essentiel, bien fait", "Highlighting, folding, search: the essentials, done well"),
                 ("Plus de 30 langages via des grammaires TextMate, pliage des blocs, commentaires et sections Markdown depuis la marge, minimap, recherche et remplacement avec expressions régulières, édition multi-curseurs, formatage par prettier, rustfmt et autres outils externes.", "30+ languages through TextMate grammars, folding of blocks, comments and Markdown sections from the gutter, a minimap, search and replace with regular expressions, multiple cursors, formatting through prettier, rustfmt and other external tools."),
                 [("Thème sombre et clair, police au choix parmi celles à chasse fixe installées", "Dark and light themes, text font chosen among the installed fixed-width fonts"),
                  ("Duplication, déplacement, commentaire et indentation de lignes", "Line duplication, moving, commenting and indenting"),
                  ("Aperçu Markdown et arbre JSON", "Markdown preview and JSON tree")],
                 ("guide-editing.html", "Voir l'édition", "See editing"),
                 slot("editing.png", "Fenêtre avec un fichier Markdown en mode « Partagé » (texte et aperçu côte à côte), pliage visible dans la marge, minimap à droite.", "Window with a Markdown file in Split mode (text and preview side by side), folding chevrons in the gutter, minimap on the right.", page="index.html", title="Édition")))
    rows.append((False, "Recherche", ("Tout trouver depuis un seul champ", "Find everything from one field"),
                 ("« Rechercher partout » réunit fichiers, classes, symboles, actions, texte des fichiers et ce que proposent les plugins (branches, commits…). Tapez <code>nom:ligne</code> pour aller à une ligne, <code>/</code> pour les réglages et les pages.", "“Search everywhere” gathers files, classes, symbols, actions, text in files and what plugins offer (branches, commits…). Type <code>name:line</code> to go to a line, <code>/</code> for settings and pages."),
                 [("Onglets Tout, Classes, Fichiers, Symboles, Actions, Texte, Plugins", "Tabs All, Classes, Files, Symbols, Actions, Text, Plugins"),
                  ("Chaque action a son raccourci, que vous pouvez changer", "Every action has a shortcut you can change"),
                  ("Les menus et actions s'éteignent quand elles ne peuvent rien faire", "Menus and actions switch off when they cannot do anything")],
                 ("guide-search.html", "Voir la recherche", "See search"),
                 slot("search-everywhere.png", "La fenêtre « Rechercher partout » ouverte sur l'onglet Tout, avec une saisie et des résultats de plusieurs types.", "The Search everywhere window open on the All tab, with a query and results of several kinds.", page="index.html", title="Rechercher partout")))
    rows.append((True, "Espace de travail", ("Notes jetables, signets, projets", "Scratch files, bookmarks, projects"),
                 ("Le bouton « Scratch » sous la liste des fichiers crée des notes et essais qui s'enregistrent en tapant et restent jusqu'à leur suppression. Le panneau des signets liste chaque fichier marqué avec ses lignes ; un ruban jaune apparaît dans la marge et la minimap.", "The “Scratch” button under the file list creates notes and trials saved as you type that stay until you delete them. The bookmarks panel lists each marked file with its lines; a yellow ribbon shows in the gutter and the minimap."),
                 [("Plusieurs projets ouverts dans la barre latérale", "Several projects open in the sidebar"),
                  ("Fenêtre scindée à droite ou en dessous", "Split the window right or below"),
                  ("Menu contextuel dans les zones vides des panneaux", "Context menu on the empty room of the panels")],
                 ("guide-workspace.html", "Voir l'espace de travail", "See the workspace"),
                 slot("workspace.png", "Barre latérale avec le panneau Projets déplié et le panneau Scratch ouvert, deux fichiers côte à côte en fenêtre scindée.", "Sidebar with the Projects panel expanded and the Scratch panel open, two files side by side in a split window.", page="index.html", title="Espace de travail")))
    out = ""
    for rev, eb, title, text, bullets, link, media in rows:
        li = "".join(f"<li>{L(*b)}</li>" for b in bullets)
        out += (f'<div class="show{" rev" if rev else ""}"><div class="show-text"><span class="eyebrow">{eb}</span>'
                f'<h3>{L(*title)}</h3><p>{L(*text)}</p><ul>{li}</ul>'
                f'<a class="btn small" href="{link[0]}">{L(link[1], link[2])} →</a></div>{media}</div>')
    return f'<section class="block" style="padding-top:24px"><div class="wrap">{out}</div></section>'


def plugin_strip():
    cards = ""
    for p in PLUGINS:
        cards += (f'<a class="card" href="plugin-{p["slug"]}.html"><div class="pi">{icon(p["icon"])}</div>'
                  f'<h3>{L(*p["name"])}</h3><p>{L(*p["tag"])}</p><span class="more">{L("Découvrir", "Discover")} →</span></a>')
    cards += (f'<a class="card" href="plugins.html"><div class="pi">{icon("plug")}</div><h3>{L("Le vôtre", "Your own")}</h3>'
              f'<p>{L("Un plugin est un programme et un fichier plugin.json, dans le langage de votre choix.", "A plugin is a program and a plugin.json file, in the language of your choice.")}</p>'
              f'<span class="more">{L("Créer un plugin", "Write a plugin")} →</span></a>')
    return f'''<section class="block" style="background:var(--side);border-block:1px solid var(--line)"><div class="wrap">
<div class="sec-head"><span class="eyebrow">Plugins</span>
<h2>{L("Un éditeur simple, des plugins pour le reste.", "A plain editor, plugins for the rest.")}</h2>
<p class="lead">{L("Neuf plugins officiels, chacun dans son propre dépôt, signés et installables depuis Préférences › Plugins. Les outils que vous utilisiez avant l'arrivée des plugins s'installent d'eux-mêmes au premier lancement.", "Nine official plugins, each in its own repository, signed and installable from Preferences › Plugins. The tools you used before plugins existed install themselves on first launch.")}</p></div>
<div class="grid g3">{cards}</div>
<div class="cta-row" style="margin-top:32px"><a class="btn primary" href="extensions.html">{L("Voir tous les plugins", "See all plugins")}</a></div>
</div></section>'''


def videos():
    vs = [
        ("tour.mp4", ("Visite de 60 secondes", "A 60-second tour"),
         ("Ouvrir un projet, deux onglets, recherche partout, changer de thème, ouvrir les Préférences. Court et rythmé.", "Open a project, two tabs, search everywhere, switch theme, open the Preferences. Short and brisk.")),
        ("welcome-first-launch.mp4", ("Premier lancement", "First launch"),
         ("La fenêtre d'accueil : langue, thème, taille du texte, puis installation de plugins en un clic.", "The welcome window: language, theme, text size, then installing plugins in one click.")),
        ("plugins-install.mp4", ("Installer un plugin", "Install a plugin"),
         ("Préférences › Plugins : chercher « Git », installer, le panneau apparaît dans l'éditeur.", "Preferences › Plugins: search “Git”, install, the panel appears in the editor.")),
    ]
    cards = ""
    for name, t, d in vs:
        cards += (f'<div>{slot(name, d[0], d[1], kind="video", page="index.html", ratio="16/10", title="Ardoise")}'
                  f'<h3 style="margin-top:18px;font-size:18px">{L(*t)}</h3><p class="muted" style="font-size:15px;margin-top:4px">{L(*d)}</p></div>')
    return f'''<section class="block"><div class="wrap">
<div class="sec-head"><span class="eyebrow">{L("En vidéo", "In motion")}</span><h2>{L("Voyez-le travailler.", "See it at work.")}</h2></div>
<div class="grid g3">{cards}</div></div></section>'''


def themes():
    return f'''<section class="block tight"><div class="wrap">
<div class="sec-head"><span class="eyebrow">{L("Apparence", "Appearance")}</span><h2>{L("Sombre, clair, et vos yeux d'abord.", "Dark, light, and your eyes first.")}</h2>
<p class="lead">{L("Deux thèmes mémorisés d'un lancement à l'autre, un mode contraste élevé, la police et la taille du texte à votre main.", "Two themes remembered between launches, a high-contrast mode, the font and text size in your hands.")}</p></div>
<div class="grid g2">{slot("theme-dark.png", "La même fenêtre en thème sombre (le même fichier ouvert que dans le thème clair, pour les comparer).", "The same window in the dark theme (same open file as the light one).", page="index.html", title="Thème sombre")}
{slot("theme-light.png", "La même fenêtre en thème clair, même fichier, même position.", "The same window in the light theme, same file, same position.", page="index.html", title="Thème clair")}</div>
</div></section>'''


def faq():
    items = [
        (("Sur quels systèmes fonctionne Ardoise ?", "Which systems does Ardoise run on?"),
         ("macOS (Apple Silicon) et Linux (x86_64), en binaires natifs. Il est aussi possible de compiler depuis les sources.", "macOS (Apple Silicon) and Linux (x86_64), as native binaries. You can also build from source.")),
        (("Est-ce vraiment libre ?", "Is it really free?"),
         ("Oui : licence MIT, sans compte, sans filigrane. Le code est sur GitHub.", "Yes: MIT license, no account, no watermark. The code is on GitHub.")),
        (("Les plugins sont-ils sûrs ?", "Are plugins safe?"),
         ("Les plugins officiels sont publiés dans leur propre dépôt et signés avec la clé des versions d'Ardoise. Les commandes propres à un projet ne s'exécutent qu'une fois le dossier approuvé.", "Official plugins are published in their own repository and signed with Ardoise's release key. A project's own commands run only once you trust the folder.")),
        (("Où sont mes données ?", "Where is my data?"),
         ("La session (y compris le texte non enregistré) est dans <code>~/.local/share/ardoise/session.json</code> (<code>~/Library/Application Support/ardoise</code> sur macOS), lisible par vous seul. Supprimez ce dossier pour l'oublier.", "The session (unsaved text included) is in <code>~/.local/share/ardoise/session.json</code> (<code>~/Library/Application Support/ardoise</code> on macOS), readable by you only. Delete that folder to forget it.")),
        (("Comment tout désinstaller ?", "How do I remove everything?"),
         ("Une commande par mode d'installation, détaillée sur la page <a href=\"install.html#uninstall\">Installation</a>. Le script de désinstallation supprime aussi réglages, session, plugins et mots de passe du trousseau, sauf avec <code>--keep-data</code>.", "One command per install method, detailed on the <a href=\"install.html#uninstall\">Install</a> page. The uninstall script also removes settings, session, plugins and keychain passwords, unless you pass <code>--keep-data</code>.")),
    ]
    d = "".join(f"<details><summary>{L(*q)}</summary><div>{L(*a)}</div></details>" for q, a in items)
    return f'''<section class="block tight"><div class="wrap" style="max-width:860px"><div class="sec-head"><span class="eyebrow">FAQ</span><h2>{L("Questions fréquentes", "Frequently asked")}</h2></div>{d}</div></section>'''


def build(out):
    body = (hero() + pillars() + showcase() + themes() + plugin_strip() + videos() + faq() +
            cta_block("Prêt à essayer ?", "Ready to try it?",
                      "Une ligne dans le terminal, et Ardoise est installé.", "One line in the terminal and Ardoise is installed.",
                      [("install.html", "Installer", "Install", True), ("guide.html", "Parcourir le guide", "Browse the guide", False)]))
    page("index.html", "home",
         ("Un éditeur de texte rapide, qui s'équipe à votre mesure", "A fast text editor that grows to fit you"),
         ("Ardoise Editor : un éditeur de texte rapide et libre pour macOS et Linux, écrit en Rust, avec des plugins officiels (Git, base de données, API, terminal…).",
          "Ardoise Editor: a fast, free text editor for macOS and Linux written in Rust, with official plugins (Git, database, API, terminal…)."),
         body, out)
