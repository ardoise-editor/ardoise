# -*- coding: utf-8 -*-
from lib import *

GUIDE = [
    ("guide-start.html", "Prise en main", "Getting started", "L'interface, ouvrir des fichiers et des dossiers, les onglets et la ligne de commande.", "The interface, opening files and folders, tabs and the command line.", "01"),
    ("guide-editing.html", "Édition", "Editing", "Saisie, lignes, multi-curseurs, pliage, complétion, snippets, formatage et aperçus.", "Typing, lines, multiple cursors, folding, completion, snippets, formatting and previews.", "02"),
    ("guide-navigation.html", "Navigation", "Navigation", "Aller à la définition, précédent / suivant, marque-pages, aller au fichier, à la classe, au symbole.", "Go to definition, back / forward, bookmarks, go to file, class or symbol.", "03"),
    ("guide-search.html", "Recherche", "Search", "Rechercher et remplacer, dans les fichiers, et « Rechercher partout ».", "Find and replace, in files, and Search everywhere.", "04"),
    ("guide-workspace.html", "Espace de travail", "Workspace", "Barre latérale, projets, fichiers Scratch, marque-pages, éditeur divisé.", "Sidebar, projects, scratch files, bookmarks, split windows.", "05"),
    ("guide-languages.html", "Langages et outils", "Languages and tools", "Coloration, serveurs de langage, formatage, lint, orthographe, encodage.", "Highlighting, language servers, formatting, lint, spelling, encoding.", "06"),
    ("guide-preferences.html", "Préférences", "Preferences", "Où trouver chaque réglage, et comment le remettre à zéro.", "Where to find each setting, and how to reset it.", "07"),
    ("guide-shortcuts.html", "Raccourcis clavier", "Keyboard shortcuts", "Tous les raccourcis par défaut, et comment les changer.", "Every default shortcut, and how to change them.", "08"),
]
IDX = {g[0]: i for i, g in enumerate(GUIDE)}
FN = "guide-%s.html"


def K(*keys):
    return " ".join(f"<kbd>{k}</kbd>" for k in keys)


def shell(fn, body_sections, toc, out):
    i = IDX[fn]
    g = GUIDE[i]
    prev = (GUIDE[i - 1][0], GUIDE[i - 1][1], GUIDE[i - 1][2]) if i > 0 else ("guide.html", "Sommaire du guide", "Guide contents")
    nxt = (GUIDE[i + 1][0], GUIDE[i + 1][1], GUIDE[i + 1][2]) if i < len(GUIDE) - 1 else ("extensions.html", "Les plugins", "The plugins")
    groups = [("Cette page", "On this page", toc),
              ("Le guide", "The guide", [("guide.html", "Sommaire", "Contents")] + [(x[0], x[1], x[2]) for x in GUIDE])]
    head = (f'<section class="page-head"><div class="wrap"><div class="crumbs"><a href="index.html">{L("Accueil", "Home")}</a> / '
            f'<a href="guide.html">Guide</a> / {L(g[1], g[2])}</div><h1>{L(g[1], g[2])}</h1>'
            f'<p class="lead">{L(g[3], g[4])}</p></div></section>')
    page(fn, "guide", (g[1] + " — Guide", g[2] + " — Guide"), (g[3], g[4]),
         head + doc_layout(groups, body_sections, (prev, nxt), active=fn), out)


def S(name, fr, en, fn, kind="img", ratio="16/10", title="Ardoise"):
    return slot(name, fr, en, kind=kind, ratio=ratio, page=fn, title=title)


# ------------------------------------------------------------------ pages
def start(out):
    fn = "guide-start.html"
    c = f'''
{H(2, "Le tour de la fenêtre", "A tour of the window", "tour")}
{P("Ardoise tient en quelques zones : la <b>barre de menus</b> (Fichier, Édition, Affichage, Aide), la <b>barre latérale</b> avec ses panneaux (Fichiers, Projets, et ceux des plugins), la <b>bande d'onglets</b>, la <b>zone d'édition</b> avec sa marge (numéros de ligne, pliage, marque-pages, marques Git) et sa minimap, et la <b>barre d'état</b> en bas (position du curseur, langage, indentation, encodage, fins de ligne).", "Ardoise is a few areas: the <b>menu bar</b> (File, Edit, View, Help), the <b>sidebar</b> with its panels (Files, Projects, and those of plugins), the <b>tab strip</b>, the <b>editing area</b> with its gutter (line numbers, folding, bookmarks, Git marks) and its minimap, and the <b>status bar</b> at the bottom (cursor position, language, indentation, encoding, line endings).")}
{S("guide-window-tour.png", "La fenêtre entière avec un projet ouvert, où l'on distingue les menus, la barre latérale, les onglets, la marge, la minimap et la barre d'état.", "The whole window: menus, sidebar, tabs, gutter, minimap, status bar.", fn, title="Ardoise")}
{H(2, "Ouvrir des fichiers et des dossiers", "Opening files and folders", "open")}
{UL([("<b>Fichier › Ouvrir…</b> (" + K("Ctrl", "O") + ") ouvre un fichier ; plusieurs fichiers donnent un onglet chacun.", "<b>File › Open…</b> (" + K("Ctrl", "O") + ") opens a file; several files give one tab each."),
     ("<b>Fichier › Ajouter un dossier au projet…</b> (ou <b>Ajouter un dossier</b>, au clic droit dans le panneau Projets) place un dossier dans la barre latérale : dans son arbre, un clic montre un fichier, un second le garde ouvert.", "<b>File › Add a folder to the project…</b> (or <b>Add a folder</b>, on a right click in the Projects panel) puts a folder in the sidebar: in its tree, a click shows a file, a second one keeps it open."),
     ("Depuis le terminal : <code>ardoise chemin/fichier</code>. Un fichier qui n'existe pas s'ouvre vide ; l'enregistrer le crée.", "From the terminal: <code>ardoise path/file</code>. A file that does not exist opens empty; saving creates it."),
     ("Les fichiers jusqu'à <b>64 Mo</b> s'ouvrent, en <b>UTF-8</b> ; fins de ligne (LF / CRLF) et retour final sont conservés.", "Files up to <b>64 MB</b> open, as <b>UTF-8</b>; line endings (LF / CRLF) and the final newline are kept.")])}
{H(2, "Onglets", "Tabs", "tabs")}
{UL([("<b>Fermer</b> " + K("Ctrl", "F4") + ", <b>Fermer les autres onglets</b>, <b>Fermer tous les onglets</b>.", "<b>Close</b> " + K("Ctrl", "F4") + ", <b>Close other tabs</b>, <b>Close all tabs</b>."),
     ("<b>Rouvrir l'onglet fermé</b> : " + K("Ctrl", "Shift", "T") + ".", "<b>Reopen the closed tab</b>: " + K("Ctrl", "Shift", "T") + "."),
     ("Le texte <b>non enregistré est conservé</b> d'un lancement à l'autre, y compris celui des onglets sans nom.", "<b>Unsaved text is kept</b> between runs, including that of untitled tabs.")])}
{H(2, "Enregistrer", "Saving", "save")}
{P("<b>Enregistrer</b> " + K("Ctrl", "S") + ", <b>Enregistrer sous</b> " + K("Ctrl", "Shift", "S") + ", <b>Tout enregistrer</b>. Avec <b>Préférences › Éditeur › Enregistrer les fichiers automatiquement</b>, les fichiers modifiés sont écrits quelques instants après la dernière frappe. Si un fichier a changé sur le disque, la barre d'état prévient que l'enregistrement écrasera ces changements ; s'il a été supprimé ou déplacé, il sera recréé.", "<b>Save</b> " + K("Ctrl", "S") + ", <b>Save as</b> " + K("Ctrl", "Shift", "S") + ", <b>Save all</b>. With <b>Preferences › Editor › Save files automatically</b>, modified files are written a moment after you stop typing. If a file changed on disk, the status bar warns that saving will overwrite those changes; if it was deleted or moved, it will be recreated.")}
{H(2, "La palette de commandes", "The command palette", "palette")}
{P("<b>" + K("Ctrl", "Shift", "A") + "</b> ouvre la recherche d'actions : toute commande du menu, et bien d'autres (trier les lignes, changer la casse, convertir l'indentation, choisir l'encodage…), avec leur raccourci quand il existe. Si vous ne savez pas où est une fonction, cherchez-la ici.", "<b>" + K("Ctrl", "Shift", "A") + "</b> opens the search for actions: every menu command, and many more (sort lines, change case, convert indentation, set the encoding…), with their shortcut when there is one. If you don't know where a function is, look for it here.")}
{S("guide-palette.png", "La recherche d'actions ouverte avec « trier » tapé : les actions de tri de lignes et leurs raccourcis.", "The search for actions open with “sort” typed: the line-sorting actions and their shortcuts.", fn, title="Actions")}
{H(2, "Au premier lancement", "On first launch", "welcome")}
{P("Une fenêtre d'accueil propose la langue, le thème, la taille du texte, l'enregistrement et le retour à la ligne, puis les plugins officiels à installer en un clic. Voir la page <a href=\"install.html#first\">Installation</a>.", "A welcome window offers language, theme, text size, saving and wrapping, then the official plugins to install in one click. See the <a href=\"install.html#first\">Install</a> page.")}
'''
    toc = [("#tour", "La fenêtre", "The window"), ("#open", "Ouvrir", "Opening"), ("#tabs", "Onglets", "Tabs"),
           ("#save", "Enregistrer", "Saving"), ("#palette", "Palette de commandes", "Command palette"), ("#welcome", "Premier lancement", "First launch")]
    shell(fn, c, toc, out)


def editing(out):
    fn = "guide-editing.html"
    c = f'''
{H(2, "Saisie", "Typing", "typing")}
{P("Retrait automatique, fermeture automatique des parenthèses et des guillemets, largeur de tabulation et retour à la ligne se règlent dans <b>Préférences › Éditeur</b> (blocs Affichage, Indentation, Saisie). La barre d'état indique l'indentation du fichier (espaces ou tabulations) et prévient quand les fins de ligne sont mélangées.", "Auto-indent, bracket and quote completion, tab width and word wrap are set in <b>Preferences › Editor</b> (Display, Indentation, Typing blocks). The status bar shows the file's indentation (spaces or tabs) and warns when line endings are mixed.")}
{H(2, "Éditer des lignes", "Line editing", "lines")}
{table([("Action", "Action"), ("Raccourci", "Shortcut")], [
    [("Dupliquer la ligne", "Duplicate line"), K("Ctrl", "D")], [("Supprimer la ligne", "Delete line"), K("Ctrl", "Y")],
    [("Déplacer la ligne vers le haut / le bas", "Move line up / down"), K("Alt", "Shift", "↑") + " / " + K("Alt", "Shift", "↓")],
    [("Commenter / décommenter", "Comment / uncomment"), K("Ctrl", "/")], [("Aller à la ligne", "Go to line"), K("Ctrl", "G")]])}
{P("Dans la palette (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd>) : trier les lignes (croissant, décroissant, sans la casse, numérique), supprimer les doublons, inverser, joindre, supprimer les espaces de fin, et convertir la casse (MAJUSCULES, minuscules, Titre, camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case).", "In the palette (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd>): sort lines (ascending, descending, ignore case, numeric), remove duplicate lines, reverse, join, trim trailing whitespace, and convert case (UPPERCASE, lowercase, Title Case, camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case).")}
{S("guide-lines.mp4", "Dupliquer une ligne, la déplacer avec Alt+Maj+↑, commenter un bloc, puis trier des lignes depuis la palette.", "Duplicate a line, move it with Alt+Shift+↑, comment a block, then sort lines from the palette.", fn, kind="video")}
{H(2, "Plusieurs curseurs", "Multiple cursors", "cursors")}
{table([("Action", "Action"), ("Raccourci", "Shortcut")], [
    [("Ajouter l'occurrence suivante", "Add the next occurrence"), K("Alt", "J")], [("Sélectionner toutes les occurrences", "Select all occurrences"), K("Ctrl", "Alt", "Shift", "J")],
    [("Ajouter un curseur au-dessus / en dessous", "Add a cursor above / below"), K("Ctrl", "Alt", "↑") + " / " + K("Ctrl", "Alt", "↓")]])}
{S("guide-multicursor.mp4", "Un nom de variable : Alt+J trois fois, puis le renommer partout d'un coup.", "A variable name: Alt+J three times, then rename them all at once.", fn, kind="video")}
{H(2, "Pliage de code", "Code folding", "fold")}
{P("Des chevrons dans la marge plient les blocs, les commentaires et les sections Markdown. <kbd>Ctrl</kbd>+<kbd>.</kbd> plie ou déplie le bloc courant ; <b>Édition › Replier / déplier</b> permet aussi de tout replier ou de tout déplier.", "Chevrons in the gutter fold blocks, comments and Markdown sections. <kbd>Ctrl</kbd>+<kbd>.</kbd> folds or unfolds the current block; <b>Edit › Fold / Unfold</b> also folds or unfolds everything.")}
{H(2, "Complétion et snippets", "Completion and snippets", "completion")}
{UL([("La liste de complétion s'ouvre en tapant (ou seulement au raccourci, selon <b>Préférences › Serveurs de langage</b>) ; <kbd>Ctrl</kbd>+<kbd>Space</kbd> la force, <kbd>Ctrl</kbd>+<kbd>P</kbd> montre les paramètres d'une fonction.", "The completion list opens as you type (or only with the shortcut, per <b>Preferences › Language servers</b>); <kbd>Ctrl</kbd>+<kbd>Space</kbd> forces it, <kbd>Ctrl</kbd>+<kbd>P</kbd> shows a function's parameters."),
     ("<b>Snippets</b> : une page à part dans les Préférences explique leur fonctionnement et comment en écrire ; chacun s'active, se désactive ou se supprime ; deux exemples sont fournis. <kbd>Tab</kbd> juste après le déclencheur exact insère le snippet.", "<b>Snippets</b>: a page of their own in the Preferences explains how they work and how to make one; each can be turned on or off, or deleted; two examples come with a fresh install. <kbd>Tab</kbd> right after the exact trigger inserts it.")])}
{S("guide-completion.png", "La liste de complétion ouverte dans un fichier de code, avec un élément sélectionné et le résumé de ses paramètres.", "The completion list open in a code file, one item selected, with its parameter summary.", fn, title="Complétion")}
{H(2, "Formater le code", "Formatting code", "format")}
{P("<b>Édition › Formater le code</b> (<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>L</kbd>) appelle l'outil externe du langage (prettier, rustfmt, black…) dans le dossier du fichier : la configuration du projet s'applique. Rien n'est téléchargé pour vous : l'outil doit être installé.", "<b>Edit › Format Code</b> (<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>L</kbd>) calls the language's external tool (prettier, rustfmt, black…) in the file's folder, so the project's own configuration applies. Nothing is downloaded for you: the tool must be installed.")}
{H(2, "Aperçus Markdown et JSON", "Markdown and JSON previews", "preview")}
{P("Les fichiers Markdown et JSON proposent des modes : <b>Brut</b>, <b>Aperçu</b>, <b>Divisé</b> (texte et aperçu côte à côte) et, pour JSON, <b>Arbre</b>. Un JSON invalide est signalé ; un document trop grand est tronqué à l'affichage.", "Markdown and JSON files offer modes: <b>Raw</b>, <b>Preview</b>, <b>Split</b> (text and preview side by side) and, for JSON, <b>Tree</b>. Invalid JSON is flagged; a document that is too large is truncated in display.")}
{S("guide-preview.png", "Un README.md en mode Divisé (texte à gauche, rendu à droite) avec les boutons de mode visibles.", "A README.md in Split mode (text on the left, render on the right) with the mode buttons visible.", fn, title="README.md")}
'''
    toc = [("#typing", "Saisie", "Typing"), ("#lines", "Lignes", "Lines"), ("#cursors", "Multi-curseurs", "Multiple cursors"), ("#fold", "Pliage", "Folding"),
           ("#completion", "Complétion et snippets", "Completion, snippets"), ("#format", "Formatage", "Formatting"), ("#preview", "Aperçus", "Previews")]
    shell(fn, c, toc, out)


def navigation(out):
    fn = "guide-navigation.html"
    c = f'''
{H(2, "Aller à la définition", "Go to definition", "def")}
{P("<b>Ctrl+clic</b> (Cmd+clic sur macOS) sur une fonction, une variable, une classe ou une interface saute à l'endroit où elle est définie. Sur la définition elle-même, la même action liste les endroits où elle est utilisée. Ardoise ne répond que lorsqu'il est sûr de lui.", "<b>Ctrl+click</b> (Cmd+click on macOS) on a function, variable, class or interface jumps to where it is defined. On the definition itself, the same action lists where it is used. Ardoise only answers when it is sure.")}
{S("goto-definition.mp4", "Ctrl+clic sur un appel de fonction → le fichier de définition s'ouvre à la bonne ligne ; Ctrl+clic sur la définition → la liste des usages.", "Ctrl+click a call → the definition opens at the right line; Ctrl+click the definition → the list of uses.", fn, kind="video")}
{H(2, "Précédent et suivant", "Back and forward", "back")}
{P("<kbd>Alt</kbd>+<kbd>←</kbd> / <kbd>Alt</kbd>+<kbd>→</kbd> (ou les boutons latéraux de la souris ; <kbd>Cmd</kbd>+<kbd>[</kbd> / <kbd>]</kbd> sur macOS) reviennent sur les endroits d'où vous avez sauté, d'un onglet à l'autre.", "<kbd>Alt</kbd>+<kbd>←</kbd> / <kbd>Alt</kbd>+<kbd>→</kbd> (or the mouse side buttons; <kbd>Cmd</kbd>+<kbd>[</kbd> / <kbd>]</kbd> on macOS) walk back and forth through the places you jumped from, across tabs.")}
{H(2, "Avec ou sans serveur de langage", "With or without a language server", "lsp")}
{P("Les serveurs de langage sont <b>facultatifs</b> : <b>Préférences › Serveurs de langage</b> installe (et retire) celui d'un langage sans droits administrateur, et la barre d'état le propose à l'ouverture d'un fichier. Sans serveur, les définitions sont trouvées par une recherche de motifs prudente. Pendant qu'un serveur charge le projet, des noms peuvent manquer : la liste se met à jour.", "Language servers are <b>optional</b>: <b>Preferences › Language servers</b> installs (and removes) the one of a language without administrator rights, and the status bar offers it when you open a file. Without one, definitions are found by a careful pattern search. While a server loads the project, names may be missing: the list updates itself.")}
{H(2, "Aller à…", "Go to…", "goto")}
{table([("Action", "Action"), ("Raccourci", "Shortcut")], [
    [("Aller au fichier", "Go to file"), K("Ctrl", "Shift", "N")], [("Aller à la classe", "Go to class"), K("Ctrl", "Alt", "N")],
    [("Aller au symbole", "Go to symbol"), K("Ctrl", "Alt", "Shift", "N")], [("Aller à la ligne", "Go to line"), K("Ctrl", "G")],
    [("Renommer le symbole", "Rename symbol"), K("Shift", "F6")]])}
{P("Dans « Rechercher partout », tapez <code>nom:ligne</code> pour ouvrir un fichier à une ligne, et <code>Type.nom</code> pour un symbole.", "In Search everywhere, type <code>name:line</code> to open a file at a line, and <code>Type.name</code> for a symbol.")}
{H(2, "Marque-pages", "Bookmarks", "bookmarks")}
{P("Un marque-page se pose depuis le menu contextuel du fichier (liste des fichiers) ou de l'éditeur (une ligne) : <kbd>F11</kbd> le pose ou le retire, <kbd>F2</kbd> / <kbd>Shift</kbd>+<kbd>F2</kbd> passent au suivant / précédent. Les marque-pages s'affichent en ruban jaune dans la marge et un repère jaune dans la minimap. <kbd>Shift</kbd>+<kbd>F11</kbd> ouvre le panneau des marque-pages : chaque fichier marqué avec ses lignes (<code>58: du code</code>) ; double-clic pour ouvrir à la ligne, clic droit pour supprimer.", "A bookmark is set from the context menu of the file (file list) or of the editor (a line): <kbd>F11</kbd> sets or removes it, <kbd>F2</kbd> / <kbd>Shift</kbd>+<kbd>F2</kbd> go to the next / previous. Bookmarks show as a yellow ribbon in the gutter and a yellow mark in the minimap. <kbd>Shift</kbd>+<kbd>F11</kbd> opens the bookmarks panel: every marked file with its lines (<code>58: some code</code>); double-click to open at the line, right click to delete.")}
{S("guide-bookmarks.png", "Le panneau des marque-pages avec deux fichiers et leurs lignes, et le ruban jaune visible dans la marge de l'éditeur à côté.", "The bookmarks panel with two files and their lines, and the yellow ribbon visible in the editor gutter beside it.", fn, title="Marque-pages")}
'''
    toc = [("#def", "Définition", "Definition"), ("#back", "Précédent / suivant", "Back / forward"), ("#lsp", "Serveurs de langage", "Language servers"),
           ("#goto", "Aller à…", "Go to…"), ("#bookmarks", "Marque-pages", "Bookmarks")]
    shell(fn, c, toc, out)


def search(out):
    fn = "guide-search.html"
    c = f'''
{H(2, "Rechercher et remplacer", "Find and replace", "find")}
{P("<b>Édition › Rechercher › Rechercher / Remplacer…</b> (<kbd>Ctrl</kbd>+<kbd>F</kbd>) cherche dans le fichier courant, avec <b>expressions régulières</b> et option de casse.", "<b>Edit › Find › Find / Replace…</b> (<kbd>Ctrl</kbd>+<kbd>F</kbd>) searches the current file, with <b>regular expressions</b> and a case option.")}
{S("guide-find.png", "La barre de recherche/remplacement ouverte avec une expression régulière, les occurrences surlignées dans le fichier.", "The find/replace bar open with a regular expression, matches highlighted in the file.", fn, title="Rechercher")}
{H(2, "Dans les fichiers", "In files", "infiles")}
{P("<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F</kbd> cherche dans tous les projets ouverts ; <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> remplace dans les fichiers. L'historique de ces recherches et remplacements est conservé ; il s'efface dans <b>Préférences › Éditeur › Historique</b>. Le plugin Historique local garde l'état des fichiers avant un remplacement : vous pouvez revenir en arrière.", "<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F</kbd> searches every open project; <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> replaces in files. The history of these searches and replacements is kept; clear it in <b>Preferences › Editor › History</b>. The Local History plugin keeps the files' state before a replace: you can go back.")}
{H(2, "Rechercher partout", "Search everywhere", "everywhere")}
{P("Un seul champ pour tout trouver. Les onglets filtrent par type : <b>Tout</b>, <b>Classes</b>, <b>Fichiers</b>, <b>Symboles</b>, <b>Actions</b>, <b>Texte</b> (dans les fichiers) et <b>Plugins</b> (ce qu'ils proposent : branches, commits…).", "One field to find everything. The tabs filter by kind: <b>All</b>, <b>Classes</b>, <b>Files</b>, <b>Symbols</b>, <b>Actions</b>, <b>Text</b> (in files) and <b>Plugins</b> (what they offer: branches, commits…).")}
{UL([("Fichiers : un nom de fichier ou de dossier, ou <code>nom:ligne</code> pour aller à une ligne.", "Files: a file or folder name, or <code>name:line</code> to go to a line."),
     ("Symboles : fonction, méthode, champ, variable… (<code>Type.nom</code>).", "Symbols: function, method, field, variable… (<code>Type.name</code>)."),
     ("Actions : une action ou un réglage ; tapez <code>/</code> pour lister les réglages et les pages.", "Actions: an action or a setting; type <code>/</code> to list settings and pages."),
     ("<kbd>Entrée</kbd> sur un résultat le copie quand l'astuce l'indique ; « Ouvrir dans Résultats » garde la liste dans un onglet.", "<kbd>Enter</kbd> copies a result when the hint says so; “Open in Results” keeps the list in a tab."),
     ("Un clic droit sur une action lui donne une <b>abréviation</b> de votre choix ; la liste se gère dans <b>Préférences › Éditeur › Rechercher partout</b>.", "A right click on an action gives it an <b>abbreviation</b> of your choice; the list is managed in <b>Preferences › Editor › Search everywhere</b>.")])}
{S("guide-everywhere-tabs.mp4", "Ouvrir Rechercher partout, taper un nom, parcourir les onglets Fichiers / Symboles / Actions, ouvrir un résultat.", "Open Search everywhere, type a name, flip through the Files / Symbols / Actions tabs, open a result.", fn, kind="video")}
'''
    toc = [("#find", "Rechercher / remplacer", "Find / replace"), ("#infiles", "Dans les fichiers", "In files"), ("#everywhere", "Rechercher partout", "Search everywhere")]
    shell(fn, c, toc, out)


def workspace(out):
    fn = "guide-workspace.html"
    c = f'''
{H(2, "La barre latérale", "The sidebar", "sidebar")}
{P("<kbd>Ctrl</kbd>+<kbd>B</kbd> l'affiche ou la masque. Elle est large par défaut (360 px) pour que les panneaux des plugins montrent leurs boutons en entier ; son contenu se règle dans <b>Préférences › Éditeur › Panneaux latéraux</b>. Les panneaux de chaque plugin installé sont listés dans <b>Affichage › Panneau</b>.", "<kbd>Ctrl</kbd>+<kbd>B</kbd> shows or hides it. It is wide by default (360 px) so plugin panels show their buttons whole; its content is set in <b>Preferences › Editor › Side panels</b>. The panels of every installed plugin are listed in <b>View › Panel</b>.")}
{H(2, "Fichiers et projets", "Files and projects", "projects")}
{UL([("<b>Fichiers</b> : les fichiers ouverts, pour passer de l'un à l'autre. Clic droit dans la zone vide : Nouveau fichier, Ouvrir…, Ajouter un dossier au projet…, Tout enregistrer, Fermer tous les onglets.", "<b>Files</b>: the open files, to switch between them. Right click on the empty room: New file, Open…, Add a folder to the project…, Save all, Close All Tabs."),
     ("<b>Projets</b> : un ou plusieurs dossiers en arbre. Clic droit dans la zone vide : Ajouter un dossier, Tout replier, Actualiser, Masquer / Afficher ce que Git ignore.", "<b>Projects</b>: one or several folders as a tree. Right click on the empty room: Add a folder, Fold everything, Refresh, Hide / Show what Git ignores."),
     ("Un dossier doit être <b>de confiance</b> avant que les outils qui lui sont propres (linters du projet, commandes de tâches) s'exécutent ; la liste se gère dans <b>Préférences › Éditeur › Dossiers de confiance</b>.", "A folder must be <b>trusted</b> before its own tools (project linters, task commands) run; the list is managed in <b>Preferences › Editor › Trusted folders</b>.")])}
{S("guide-projects.png", "La barre latérale avec le panneau Projets déplié sur deux projets, un fichier ouvert, et le menu clic droit de la zone vide.", "The sidebar with the Projects panel expanded on two projects, a file open, and the right-click menu of the empty room.", fn, title="Projets")}
{H(2, "Fichiers Scratch", "Scratch files", "scratch")}
{P("Le bouton <b>Fichiers scratch</b>, en bas de la barre d'activité (à gauche), ouvre un panneau pour créer des notes et des essais (un nom + une extension). Ils s'enregistrent au fil de la frappe, ne se ferment jamais et restent jusqu'à ce que vous les supprimiez. Leur dossier se change dans <b>Préférences › Éditeur › Fichiers temporaires</b> (il doit être vide : les fichiers qu'il contient deviendraient des fichiers Scratch).", "The <b>Scratch files</b> button, at the bottom of the activity bar (on the left), opens a panel to create notes and trials (a name + an extension). They are saved as you type, never close, and stay until you delete them. Their folder can be changed in <b>Preferences › Editor › Scratch files</b> (it must be empty: the files already in it would become scratch files).")}
{S("guide-scratch.png", "Le panneau Scratch avec trois notes (par ex. notes.md, essai.py, requete.sql) et l'une d'elles ouverte.", "The Scratch panel with three notes (e.g. notes.md, try.py, query.sql) and one of them open.", fn, title="Scratch")}
{H(2, "Éditeur divisé", "Split windows", "split")}
{P("<b>Affichage › Diviser</b> : à droite ou en bas, pour voir deux fichiers (ou deux endroits du même) côte à côte. <b>Fermer cette division</b> se trouve au même endroit et n'est actif que lorsque l'éditeur est divisé.", "<b>View › Split</b>: right or below, to see two files (or two places of the same) side by side. <b>Close this split</b> is in the same place and is active only when the editor is split.")}
{S("guide-split.png", "Deux fichiers côte à côte, le menu Affichage › Diviser ouvert.", "Two files side by side in a vertical split, the View › Split menu open.", fn, title="Diviser")}
{H(2, "Session et données", "Session and data", "session")}
{P("Ardoise reprend où vous en étiez : fichiers ouverts, position, et même le texte non enregistré. La session est dans <code>~/.local/share/ardoise/session.json</code> (<code>~/Library/Application Support/ardoise</code> sur macOS), lisible par vous seul ; supprimez le dossier pour l'oublier.", "Ardoise picks up where you left off: open files, position, even unsaved text. The session is in <code>~/.local/share/ardoise/session.json</code> (<code>~/Library/Application Support/ardoise</code> on macOS), readable by you only; delete the folder to forget it.")}
'''
    toc = [("#sidebar", "Barre latérale", "Sidebar"), ("#projects", "Fichiers et projets", "Files and projects"), ("#scratch", "Scratch", "Scratch"),
           ("#split", "Diviser", "Split"), ("#session", "Session", "Session")]
    shell(fn, c, toc, out)


def languages(out):
    fn = "guide-languages.html"
    c = f'''
{H(2, "Coloration syntaxique", "Syntax highlighting", "syntax")}
{P("Plus de 30 langages sont colorés à l'aide de grammaires TextMate. Le langage est détecté d'après l'extension ; pour le changer, utilisez <b>Affichage › Langage du fichier…</b>. Un fichier sans extension s'affiche en texte brut jusqu'à ce que vous l'enregistriez avec une extension.", "30+ languages are highlighted with TextMate grammars. The language is detected from the extension; to change it, use <b>View › File language…</b>. A file with no extension shows as plain text until you save it with one.")}
{S("guide-languages-grid.png", "Un montage de trois fenêtres côte à côte (Rust, Python, TypeScript) montrant la coloration. Facultatif : une seule capture convient.", "A montage of three windows (Rust, Python, TypeScript) showing highlighting. Optional: a single screenshot will do.", fn, title="Coloration")}
{H(2, "Serveurs de langage", "Language servers", "lsp")}
{P("Facultatifs, ils donnent des résultats exacts pour les définitions et la complétion. <b>Préférences › Serveurs de langage</b> en installe (ou retire) un par langage, sans droits administrateur ; ils vivent dans <code>~/.local/share/ardoise/lsp</code>.", "Optional, they give exact results for definitions and completion. <b>Preferences › Language servers</b> installs (or removes) one per language, without administrator rights; they live in <code>~/.local/share/ardoise/lsp</code>.")}
{table([("Installeur nécessaire", "Installer needed"), ("Langages", "Languages")], [
    ["<code>npm</code>", ("Python, TypeScript / JavaScript, Shell, PHP", "Python, TypeScript / JavaScript, Shell, PHP")],
    ["<code>rustup</code>", "Rust"], ["<code>go</code>", "Go"], ["<code>dotnet</code>", "C#"],
    [("Aucun (commande affichée)", "None (command shown)"), ("C / C++, Java, Lua, Ruby, Zig, Kotlin, Swift", "C / C++, Java, Lua, Ruby, Zig, Kotlin, Swift")]])}
{S("guide-lsp-install.png", "Préférences › Serveurs de langage avec quelques langages, l'un installé, l'autre proposant l'installation.", "Preferences › Language servers with a few languages, one installed, another offering to install.", fn, title="Serveurs de langage")}
{H(2, "Formatage", "Formatting", "format")}
{P("Voir <a href=\"guide-editing.html#format\">Édition › Formater le code</a>. Les outils (prettier, rustfmt, black…) s'exécutent dans le dossier du fichier ; ils doivent être installés.", "See <a href=\"guide-editing.html#format\">Editing › Formatting code</a>. Tools (prettier, rustfmt, black…) run in the file's folder; they must be installed.")}
{H(2, "Problèmes, lint et orthographe", "Problems, lint and spelling", "problems")}
{P("Ce que trouvent les serveurs de langage s'affiche comme des <b>problèmes</b> ; <b>Édition › Problèmes</b> permet de les rechercher, de les corriger et de passer au suivant ou au précédent. Le plugin <a href=\"plugin-lint.html\">Lint</a> y ajoute les linters du projet, et le plugin <a href=\"plugin-spell.html\">Orthographe</a> souligne les mots mal écrits.", "What language servers find shows as <b>problems</b>; <b>Edit › Problems</b> checks for them, fixes them and goes to the next or previous one. The <a href=\"plugin-lint.html\">Lint</a> plugin adds the project's linters, and the <a href=\"plugin-spell.html\">Spelling</a> plugin underlines misspelled words.")}
{H(2, "Encodage et fins de ligne", "Encoding and line endings", "encoding")}
{P("Les fichiers doivent être en UTF-8 ; les fins de ligne (LF / CRLF) et le retour final sont conservés tels quels. Si un fichier est mal lu, la palette propose « Encodage… » pour le relire dans un autre encodage (enregistrez ou abandonnez d'abord vos modifications). Les préférences règlent l'encodage et les fins de ligne des nouveaux fichiers.", "Files must be UTF-8; line endings (LF / CRLF) and the final newline are kept as they were. If a file is misread, the palette offers “Encoding…” to read it again in another encoding (save or discard your changes first). The preferences set the encoding and line endings of new files.")}
'''
    toc = [("#syntax", "Coloration", "Highlighting"), ("#lsp", "Serveurs de langage", "Language servers"), ("#format", "Formatage", "Formatting"),
           ("#problems", "Problèmes, lint", "Problems, lint"), ("#encoding", "Encodage", "Encoding")]
    shell(fn, c, toc, out)


def preferences(out):
    fn = "guide-preferences.html"
    c = f'''
{H(2, "Ouvrir les Préférences", "Opening the Preferences", "open")}
{P("<b>Fichier › Préférences…</b>. En haut, un champ de recherche trouve un réglage par son libellé ou son bloc (accents et majuscules sont sans importance). Les blocs Affichage, Indentation, Saisie, Complétion, Fichiers et Rechercher partout ont un bouton <b>Réinitialiser</b>.", "<b>File › Preferences…</b>. At the top, a search box finds a setting by its label or block (accents and capitals do not matter). The Display, Indentation, Typing, Completion, Files and Search everywhere blocks have a <b>Reset</b> button.")}
{S("guide-preferences.png", "La fenêtre des Préférences ouverte, le champ de recherche en haut et la page Éditeur affichée.", "The Preferences window open, the search box at the top and the Editor page showing.", fn, title="Préférences")}
{H(2, "Les pages", "The pages", "pages")}
{table([("Page / bloc", "Page / block"), ("Ce qu'on y règle", "What you set there")], [
    [("Éditeur — Affichage", "Editor — Display"), ("Taille et police du texte (choisie parmi les polices à chasse fixe installées), numéros de ligne, retour à la ligne, colonne guide.", "Text size and font (chosen among installed fixed-width fonts), line numbers, word wrap, guide column.")],
    [("Éditeur — Indentation, Saisie", "Editor — Indentation, Typing"), ("Largeur de tabulation, retrait automatique, fermeture des parenthèses et guillemets.", "Tab width, auto-indent, bracket and quote completion.")],
    [("Éditeur — Fichiers", "Editor — Files"), ("Enregistrement automatique, suppression des espaces de fin et retour final à l'enregistrement, encodage et fins de ligne des nouveaux fichiers.", "Automatic saving, trailing whitespace and final line break on save, encoding and line endings of new files.")],
    [("Éditeur — Rechercher partout", "Editor — Search everywhere"), ("Comportement de la recherche et abréviations données aux actions.", "Search behavior and abbreviations given to actions.")],
    [("Éditeur — Panneaux latéraux, Projets, Fichiers temporaires", "Editor — Side panels, Projects, Scratch files"), ("Panneaux de la barre latérale, éléments exclus des projets, dossier des fichiers scratch.", "Panels of the sidebar, what projects leave out, the folder of scratch files.")],
    [("Éditeur — Dossiers de confiance, Historique", "Editor — Trusted folders, History"), ("Retirer la confiance à un dossier ; effacer les fichiers récents, les historiques de recherche et de remplacement et les machines distantes récentes.", "Stop trusting a folder; clear the recent files, the search and replace histories and the recent remote machines.")],
    [("Éditeur — Langue", "Editor — Language"), ("Langue de l'interface (français / anglais).", "Language of the interface (English / French).")],
    [("Apparence", "Appearance"), ("Thème clair / sombre, <b>contraste élevé</b>.", "Light / dark theme, <b>high contrast</b>.")],
    [("Serveurs de langage", "Language servers"), ("Installer ou retirer un serveur par langage ; le bloc Complétion règle l'ouverture de la liste.", "Install or remove a server per language; the Completion block sets how the list opens.")],
    [("Git, Orthographe", "Git, Spelling"), ("« Masquer ce que Git ignore » est sur la page Git ; la langue du dictionnaire est sur la page Orthographe, qui liste les dictionnaires installés et comment en ajouter.", "“Hide what Git ignores” is on the Git page; the dictionary language is on the Spelling page, which lists installed dictionaries and how to add more.")],
    [("Snippets", "Snippets"), ("Écrire, activer, désactiver ou supprimer un snippet.", "Write, turn on or off, or delete a snippet.")],
    [("Raccourcis", "Shortcuts"), ("Tous les raccourcis, regroupés par thème, avec recherche par libellé ou par touches. Voir <a href=\"guide-shortcuts.html\">Raccourcis clavier</a>.", "Every shortcut, grouped by theme, with search by label or keys. See <a href=\"guide-shortcuts.html\">Keyboard shortcuts</a>.")],
    [("Plugins", "Plugins"), ("Installer et retirer les plugins, avec recherche ; lier un dossier pour développer un plugin ; réglages de chaque plugin.", "Install and remove plugins, with search; link a folder to develop a plugin; each plugin's settings.")],
    [("Lint", "Lint"), ("Affichage des problèmes et réglages des linters ; la page apparaît avec le plugin Lint.", "How problems show and the linters' settings; the page is there with the Lint plugin.")]])}
{H(2, "Thème et apparence", "Theme and appearance", "theme")}
{S("guide-theme-switch.mp4", "Basculer du thème sombre au thème clair dans Préférences › Apparence, puis activer le contraste élevé.", "Switch from the dark to the light theme in Preferences › Appearance, then turn on high contrast.", fn, kind="video")}
'''
    toc = [("#open", "Ouvrir", "Opening"), ("#pages", "Les pages", "The pages"), ("#theme", "Thème", "Theme")]
    shell(fn, c, toc, out)


SC = [
    (("Fichiers", "Files"), [("Nouveau fichier", "New file", ("Ctrl", "N")), ("Ouvrir", "Open", ("Ctrl", "O")), ("Enregistrer", "Save", ("Ctrl", "S")),
                            ("Enregistrer sous", "Save as", ("Ctrl", "Shift", "S")), ("Fermer l'onglet", "Close tab", ("Ctrl", "F4")),
                            ("Rouvrir l'onglet fermé", "Reopen the closed tab", ("Ctrl", "Shift", "T")), ("Quitter", "Quit", ("Ctrl", "Q"))]),
    (("Édition", "Edit"), [("Annuler", "Undo", ("Ctrl", "Z")), ("Rétablir", "Redo", ("Ctrl", "Shift", "Z")), ("Tout sélectionner", "Select all", ("Ctrl", "A")),
                           ("Dupliquer la ligne", "Duplicate line", ("Ctrl", "D")), ("Supprimer la ligne", "Delete line", ("Ctrl", "Y")),
                           ("Déplacer la ligne ↑ / ↓", "Move line ↑ / ↓", ("Alt", "Shift", "↑ / ↓")), ("Commenter / décommenter", "Toggle comment", ("Ctrl", "/")),
                           ("Rechercher / remplacer", "Find / Replace", ("Ctrl", "F"))]),
    (("Plusieurs curseurs", "Multiple cursors"), [("Ajouter l'occurrence suivante", "Add next occurrence", ("Alt", "J")), ("Toutes les occurrences", "Select all occurrences", ("Ctrl", "Alt", "Shift", "J")),
                                                 ("Curseur au-dessus", "Cursor above", ("Ctrl", "Alt", "↑")), ("Curseur en dessous", "Cursor below", ("Ctrl", "Alt", "↓"))]),
    (("Code", "Code"), [("Formater le code", "Format code", ("Ctrl", "Alt", "L")), ("Compléter le code", "Complete the code", ("Ctrl", "Space")),
                        ("Infos sur les paramètres", "Parameter info", ("Ctrl", "P")), ("Renommer le symbole", "Rename symbol", ("Shift", "F6")),
                        ("Plier / déplier", "Fold / unfold", ("Ctrl", ".")), ("Lancer une tâche", "Run a task", ("Ctrl", "Alt", "R")),
                        ("Suggestions orthographiques", "Spelling suggestions", ("Ctrl", "Alt", "S"))]),
    (("Recherche et navigation", "Search and navigation"), [("Palette d'actions", "Action palette", ("Ctrl", "Shift", "A")), ("Aller au fichier", "Go to file", ("Ctrl", "Shift", "N")),
                                                           ("Aller à la classe", "Go to class", ("Ctrl", "Alt", "N")), ("Aller au symbole", "Go to symbol", ("Ctrl", "Alt", "Shift", "N")),
                                                           ("Aller à la ligne", "Go to line", ("Ctrl", "G")), ("Chercher dans les fichiers", "Find in files", ("Ctrl", "Shift", "F")),
                                                           ("Remplacer dans les fichiers", "Replace in files", ("Ctrl", "Shift", "R")), ("Précédent / Suivant", "Back / Forward", ("Alt", "← / →"))]),
    (("Marque-pages", "Bookmarks"), [("Poser / retirer", "Toggle", ("F11",)), ("Suivant", "Next", ("F2",)), ("Précédent", "Previous", ("Shift", "F2")), ("Panneau des marque-pages", "Bookmarks panel", ("Shift", "F11"))]),
    (("Affichage et plugins", "View and plugins"), [("Barre latérale", "Sidebar", ("Ctrl", "B")), ("Terminal", "Terminal", ("Alt", "F12")), ("Panneau Git", "Git panel", ("Alt", "G")),
                                                    ("Git : commit", "Git: commit", ("Ctrl", "K")), ("Git : push", "Git: push", ("Ctrl", "Shift", "K")), ("Git : pull", "Git: pull", ("Ctrl", "T")),
                                                    ("Git : branches", "Git: branches", ("Ctrl", "Shift", "B"))]),
]


def shortcuts(out):
    fn = "guide-shortcuts.html"
    c = f'''
{H(2, "Changer un raccourci", "Changing a shortcut", "rebind")}
{P("<b>Aide › Configurer les raccourcis…</b> ou <b>Préférences › Raccourcis</b> : cliquez sur un raccourci (« Cliquer pour redéfinir »), appuyez sur la nouvelle combinaison. Une recherche trouve une action par son libellé ou par ses touches. Deux actions ne peuvent pas partager la même combinaison. Les touches affichées suivent votre système : sur macOS, <kbd>Ctrl</kbd> devient <kbd>⌘</kbd> et <kbd>Alt</kbd> devient <kbd>⌥</kbd>.", "<b>Help › Configure shortcuts…</b> or <b>Preferences › Shortcuts</b>: click a shortcut (“Click to rebind”), press the new combination. A search finds an action by its label or its keys. Two actions cannot share the same combination. Keys follow your system: on macOS, <kbd>Ctrl</kbd> becomes <kbd>⌘</kbd> and <kbd>Alt</kbd> becomes <kbd>⌥</kbd>.")}
{S("guide-shortcuts.png", "Préférences › Raccourcis avec un raccourci en cours de réassignation (« Appuyez sur une touche… »).", "Preferences › Shortcuts with a shortcut being rebound (“Press a key…”).", fn, title="Raccourcis")}
{callout("la liste ci-dessous donne les valeurs par défaut sous Linux. Quelques-unes diffèrent sur d'autres systèmes (par exemple Précédent / Suivant sur macOS, ou Compléter, qui y utilise ⌘+Maj+Espace).", "the list below gives the Linux defaults. A few differ on other systems (for example Back / Forward on macOS, or Complete, which uses ⌘+Shift+Space there).")}
'''
    toc = [("#rebind", "Changer un raccourci", "Rebinding")]
    for gi, (gt, rows) in enumerate(SC):
        gid = f"s{gi}"
        toc.append((f"#{gid}", gt[0], gt[1]))
        r = [[(a, b), " + ".join(f"<kbd>{k}</kbd>" for k in ks)] for a, b, ks in rows]
        c += f'<h2 id="{gid}">{L(*gt)}</h2>' + table([("Action", "Action"), ("Raccourci", "Shortcut")], r)
    shell(fn, c, toc, out)


def hub(out):
    cards = ""
    for fn, fr, en, dfr, den, n in GUIDE:
        cards += f'<a class="card" href="{fn}"><span class="num">{n}</span><h3>{L(fr, en)}</h3><p>{L(dfr, den)}</p><span class="more">{L("Lire", "Read")} →</span></a>'
    cards += (f'<a class="card" href="extensions.html"><span class="num">09</span><h3>Plugins</h3><p>{L("Git, GitHub, GitLab, base de données, API, lint, orthographe, historique, tâches, fichiers distants, terminal.", "Git, GitHub, GitLab, database, API, lint, spelling, history, tasks, remote files, terminal.")}</p><span class="more">{L("Découvrir", "Discover")} →</span></a>')
    quick = table([("Je veux…", "I want to…"), ("Raccourci", "Shortcut")], [
        [("Trouver n'importe quelle action", "Find any action"), K("Ctrl", "Shift", "A")], [("Ouvrir un fichier par son nom", "Open a file by name"), K("Ctrl", "Shift", "N")],
        [("Aller à la définition", "Go to definition"), "<kbd>Ctrl</kbd> + clic"], [("Chercher dans le fichier", "Find in the file"), K("Ctrl", "F")],
        [("Chercher dans les projets", "Find in the projects"), K("Ctrl", "Shift", "F")], [("Formater le code", "Format code"), K("Ctrl", "Alt", "L")]])
    body = f'''<section class="page-head"><div class="wrap"><div class="crumbs"><a href="index.html">{L("Accueil", "Home")}</a> / Guide</div>
<h1>{L("Le guide d'Ardoise", "The Ardoise guide")}</h1>
<p class="lead">{L("Toutes les fonctionnalités, expliquées et illustrées : de la prise en main aux raccourcis, en passant par la navigation, la recherche et les réglages.", "Every feature, explained and illustrated: from getting started to shortcuts, through navigation, search and settings.")}</p></div></section>
<section class="block tight" style="padding-top:24px"><div class="wrap"><div class="grid g3">{cards}</div></div></section>
<section class="block tight"><div class="wrap split eq"><div class="prose" style="max-width:none"><span class="eyebrow">{L("Pour commencer", "To begin with")}</span>
<h2 style="margin-top:12px">{L("Six gestes à retenir", "Six moves to remember")}</h2>{quick}</div>
{slot("tour.mp4", "Montage rapide (15 s) : palette d'actions, aller à la définition, recherche partout, marque-page, thème. Même contenu que la visite de l'accueil, en plus court.", "Quick montage (15 s): action palette, go to definition, search everywhere, bookmark, theme.", kind="video", page="guide.html", title="Ardoise")}
</div></section>'''
    page("guide.html", "guide", ("Guide", "Guide"),
         ("Documentation d'Ardoise Editor : prise en main, édition, navigation, recherche, espace de travail, langages, préférences et raccourcis.",
          "Ardoise Editor documentation: getting started, editing, navigation, search, workspace, languages, preferences and shortcuts."), body, out)


def build(out):
    hub(out)
    for f in (start, editing, navigation, search, workspace, languages, preferences, shortcuts):
        f(out)
