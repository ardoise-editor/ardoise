# -*- coding: utf-8 -*-
"""The eleven official plugins (content taken from each repository's README and plugin.json)."""

PLUGINS = [
 dict(
  slug="git", repo="plugin-git", ver="1.4.2", icon="git", name=("Git", "Git"),
  tag=("Git complet, dans un panneau sous l'éditeur.", "Full Git, in a panel under the editor."),
  intro=("Commit, remisage, historique avec son graphe, branches, push et pull, blame, diffs et conflits : tout ce qu'on fait au quotidien avec Git, sans quitter l'éditeur — jusqu'au rebase interactif.",
         "Commit, stash, log with its graph, branches, push and pull, blame, diffs and conflicts: everything you do with Git every day, without leaving the editor — down to the interactive rebase."),
  groups=[
   (("Commit", "Commit"), [
     ("Fichiers indexés et modifiés : indexer, désindexer, annuler, supprimer, remiser ou ignorer un fichier.", "Staged and changed files: stage, unstage, roll back, delete, stash or ignore a file."),
     ("Commit avec amend, sign-off, sans les hooks, auteur au choix ; commit et push en un geste ; annuler le dernier commit.", "Commit with amend, sign-off, skip the hooks, a chosen author; commit and push in one go; undo the last commit."),
     ("Conflits d'un merge, rebase, cherry-pick ou revert : poursuivre, passer ou abandonner.", "Conflicts of a merge, rebase, cherry-pick or revert: continue, skip or abort.")]),
   (("Historique (log)", "Log"), [
     ("L'historique avec son graphe : branche courante, toutes les branches, une branche ou un tag, le reflog, ou un fichier.", "History with its graph: current branch, all branches, a branch or tag, the reflog, or a file."),
     ("Recherche par message, auteur, contenu, fichier ou date, avec des filtres.", "Search by message, author, content, file or date, with filters."),
     ("Sur les commits choisis : checkout, branche, tag, reset, revert, cherry-pick, reword, squash, drop, fixup, rebase interactif, comparer, patch.", "On picked commits: checkout, branch, tag, reset, revert, cherry-pick, reword, squash, drop, fixup, interactive rebase, compare, patch.")]),
   (("Branches et remisages", "Branches and stashes"), [
     ("Branches locales et distantes, tags : checkout, merge, rebase, pull, fetch, push, force push, renommer, supprimer, comparer avec l'arbre de travail.", "Local and remote branches and tags: checkout, merge, rebase, pull, fetch, push, force push, rename, delete, compare with the working tree."),
     ("Remisage (en gardant l'index, ou l'index seul), appliquer, pop, supprimer, créer une branche depuis un remisage.", "Stash (keeping the index, or the index only), apply, pop, drop, make a branch of a stash.")]),
   (("GitHub et GitLab", "GitHub and GitLab"), [
     ("Les plugins officiels <a href=\"plugin-github.html\">GitHub</a> et <a href=\"plugin-gitlab.html\">GitLab</a> ajoutent chacun un onglet au panneau Git : avec eux, un seul bouton Git dans la barre d'activité. Ils fonctionnent aussi sans le plugin Git.", "The official <a href=\"plugin-github.html\">GitHub</a> and <a href=\"plugin-gitlab.html\">GitLab</a> plugins each add a tab to the Git panel: with them, a single Git button in the activity bar. They work without the Git plugin too."),
     ("GitHub : pull requests (créer, lire, commenter, approuver, fusionner), Actions et leurs journaux, étiquettes.", "GitHub: pull requests (make, read, comment, approve, merge), Actions and their logs, tags."),
     ("GitLab : merge requests (créer, lire, commenter, approuver, fusionner), pipelines et leurs journaux, étiquettes.", "GitLab: merge requests (make, read, comment, approve, merge), pipelines and their logs, tags.")]),
   (("Dans l'éditeur", "In the editor"), [
     ("Couleurs des fichiers modifiés dans l'arbre et sur les onglets ; lignes modifiées marquées dans la marge.", "Colors of changed files in the tree and on the tabs; changed lines marked beside the text."),
     ("Blame dans la marge (« Git : annoter (blame) »), diffs en onglets où l'on indexe ou annule des blocs, conflits résolus dans une vue à trois colonnes.", "Blame in the gutter (“Git: Annotate (Blame)”), diffs in tabs where blocks are staged or put back, conflicts settled in a three-column merge."),
     ("Branches et commits dans « Rechercher partout » (onglet Plugins) ; outils : comparer/récupérer un fichier d'une révision, historique d'un fichier ou de lignes, remotes, patches, nettoyage, sous-modules, worktrees, clone, et fetch/pull/push de tous les dépôts des projets.", "Branches and commits in Search Everywhere (Plugins tab); tools: compare with or get from a revision, history of a file or selected lines, remotes, patches, clean, submodules, worktrees, clone, and fetch/pull/push of every repository of the projects.")]),
  ],
  needs=[("<code>git</code> doit être installé sur la machine.", "<code>git</code> must be installed on the machine.")],
  settings=("Préférences › Plugins : manière de faire les pull et les merges, fréquence du fetch automatique.", "Preferences › Plugins: how Pull and merges are made, and how often to fetch."),
  keys=[(("Alt", "G"), "Afficher le panneau Git", "Show the Git panel"), (("Ctrl", "K"), "Commit", "Commit"),
        (("Ctrl", "Shift", "K"), "Push", "Push"), (("Ctrl", "T"), "Pull", "Pull"), (("Ctrl", "Shift", "B"), "Branches", "Branches")],
  media=[("plugin-git-commit.png", "Panneau Git, onglet Commit : fichiers modifiés, message en cours de saisie, diff d'un fichier ouvert à côté.", "Git panel, Commit tab: changed files, a message being typed, a file diff open beside it.", "img", "16/10"),
         ("plugin-git-log.png", "Onglet Log : le graphe d'historique avec plusieurs branches, un commit sélectionné et ses fichiers.", "Log tab: the history graph with several branches, one commit selected and its files.", "img", "16/10"),
         ("plugin-git-merge.png", "Résolution d'un conflit dans la vue à trois colonnes.", "A conflict settled in the three-column merge.", "img", "16/10"),
         ("plugin-git-tabs.png", "Le panneau Git avec les onglets Git, GitHub et GitLab, et un seul bouton Git dans la barre d'activité.", "The Git panel with the Git, GitHub and GitLab tabs, and a single Git button in the activity bar.", "img", "16/10"),
         ("plugin-git-demo.mp4", "Parcours complet : modifier un fichier, voir les lignes marquées, indexer, commit et push depuis le panneau.", "Complete flow: edit a file, see the marked lines, stage, commit and push from the panel.", "video", "16/10")],
 ),
 dict(
  slug="github", repo="plugin-github", ver="1.0.1", icon="github", name=("GitHub", "GitHub"),
  tag=("Pull requests, Actions et étiquettes, dans un onglet du panneau Git.", "Pull requests, Actions and tags, in a tab of the Git panel."),
  intro=("GitHub (github.com ou GitHub Enterprise) dans l'éditeur, en onglet du panneau Git. Le plugin fonctionne avec ou sans le <a href=\"plugin-git.html\">plugin Git</a>. Le dépôt est celui du remote <code>origin</code>.",
         "GitHub (github.com or GitHub Enterprise) in the editor, as a tab of the Git panel. The plugin works with or without the <a href=\"plugin-git.html\">Git plugin</a>. The repository is the one of the <code>origin</code> remote."),
  groups=[
   (("Pull requests", "Pull requests"), [
     ("Les pull requests ouvertes, fusionnées ou fermées ; en créer une, en brouillon ou non, depuis la branche courante.", "The open, merged or closed pull requests; make one, draft or not, from the branch checked out."),
     ("En lire une : description, fichiers et leurs diffs dans un onglet, commentaires et relectures (commenter, répondre sur des lignes), vérifications.", "Read one: its description, its files and their diffs in a tab, its comments and reviews (comment, answer on lines), its checks."),
     ("Approuver ou demander des changements, fusionner (merge ou squash, en supprimant la branche si vous le cochez), fermer, rouvrir. GitHub ne permet pas de supprimer une pull request.", "Approve or request changes, merge (merge or squash, deleting the branch when you tick it), close, reopen. GitHub does not delete pull requests."),
     ("La fusion porte sur le commit que vous avez vu : si la branche a bougé depuis, GitHub la refuse.", "Merging asks for the commit you saw: if the branch moved since, GitHub refuses.")]),
   (("Actions et étiquettes", "Actions and tags"), [
     ("Les dernières exécutions, leurs jobs et journaux ; lancer un workflow à la main ; relancer les jobs en échec ou un seul job, annuler ou supprimer une exécution.", "The latest runs, their jobs and logs; start a workflow by hand; rerun failed jobs or a single job, cancel or delete a run."),
     ("Étiquettes : les lister, en créer (annotées, avec un message), les supprimer.", "Tags: list, create (annotated with a message), delete.")]),
   (("Connexion", "Signing in"), [
     ("Pour chaque serveur, le panneau demande un jeton d'accès personnel (un jeton « classic » avec les droits <code>repo</code> et <code>workflow</code>).", "For each server the panel asks for a personal access token (a classic token with the <code>repo</code> and <code>workflow</code> scopes)."),
     ("Il est gardé dans le trousseau du système (ou, à défaut, dans un fichier lisible par vous seul) et n'est <b>envoyé qu'au serveur pour lequel vous l'avez saisi</b>, en HTTPS.", "It is kept in the system keychain (or a file only you can read when there is none) and <b>sent only to the server you typed it for</b>, over HTTPS.")]),
  ],
  needs=[("<code>git</code> sur la machine et Ardoise 2.8 ou plus récent.", "<code>git</code> on the machine and Ardoise 2.8 or later.")],
  settings=("La page de réglages du plugin, dans les Préférences, concerne les serveurs connectés.", "The settings page of the plugin, in the Preferences, is about the servers signed in to."),
  keys=[],
  media=[("plugin-github-connect.png", "L'onglet GitHub du panneau Git avant la connexion : le jeton demandé pour github.com.", "The GitHub tab of the Git panel before signing in: the token asked for github.com.", "img", "16/10")],
 ),
 dict(
  slug="gitlab", repo="plugin-gitlab", ver="1.0.1", icon="merge", name=("GitLab", "GitLab"),
  tag=("Merge requests, pipelines et étiquettes, dans un onglet du panneau Git.", "Merge requests, pipelines and tags, in a tab of the Git panel."),
  intro=("GitLab (gitlab.com ou votre serveur) dans l'éditeur, en onglet du panneau Git. Le plugin fonctionne avec ou sans le <a href=\"plugin-git.html\">plugin Git</a>. Le projet est celui du remote <code>origin</code>.",
         "GitLab (gitlab.com or your own server) in the editor, as a tab of the Git panel. The plugin works with or without the <a href=\"plugin-git.html\">Git plugin</a>. The project is the one of the <code>origin</code> remote."),
  groups=[
   (("Merge requests", "Merge requests"), [
     ("Les merge requests ouvertes, fusionnées ou fermées ; en créer une depuis la branche courante.", "The open, merged or closed merge requests; make one from the branch checked out."),
     ("En lire une : description, fichiers et leurs diffs dans un onglet, commentaires (répondre, résoudre, commenter), pipeline.", "Read one: its description, its files and their diffs in a tab, its comments (answer, resolve, comment), its pipeline."),
     ("Approuver, fusionner (squash, suppression de la branche source, une fois le pipeline réussi), fermer, rouvrir, supprimer. GitLab réserve la suppression aux propriétaires et administrateurs ; les autres ferment.", "Approve, merge (squash, delete the source branch, once the pipeline passes), close, reopen, delete. GitLab allows deleting to owners and administrators only; others close."),
     ("La fusion porte sur le commit que vous avez vu : si la branche a bougé depuis, GitLab la refuse.", "Merging asks for the commit you saw: if the branch moved since, GitLab refuses.")]),
   (("Pipelines et étiquettes", "Pipelines and tags"), [
     ("Les derniers pipelines, leurs jobs et journaux ; en lancer un sur une branche ou un tag ; relancer, annuler, supprimer ; relancer, annuler ou démarrer un job.", "The latest pipelines, their jobs and logs; run one on a branch or a tag; retry, cancel, delete; retry, cancel or start a job."),
     ("Étiquettes : les lister, en créer (annotées, avec un message), les supprimer.", "Tags: list, create (annotated with a message), delete.")]),
   (("Connexion", "Signing in"), [
     ("Pour chaque serveur, le panneau demande un jeton d'accès personnel avec le droit <code>api</code>.", "For each server the panel asks for a personal access token with the <code>api</code> scope."),
     ("Il est gardé dans le trousseau du système (ou, à défaut, dans un fichier lisible par vous seul) et n'est <b>envoyé qu'au serveur pour lequel vous l'avez saisi</b>, en HTTPS.", "It is kept in the system keychain (or a file only you can read when there is none) and <b>sent only to the server you typed it for</b>, over HTTPS.")]),
  ],
  needs=[("<code>git</code> sur la machine et Ardoise 2.8 ou plus récent.", "<code>git</code> on the machine and Ardoise 2.8 or later.")],
  settings=("La page de réglages du plugin, dans les Préférences, concerne les serveurs connectés.", "The settings page of the plugin, in the Preferences, is about the servers signed in to."),
  keys=[],
  media=[("plugin-gitlab-connect.png", "L'onglet GitLab du panneau Git avant la connexion : le jeton demandé pour gitlab.com.", "The GitLab tab of the Git panel before signing in: the token asked for gitlab.com.", "img", "16/10")],
 ),
 dict(
  slug="database", repo="plugin-database", ver="1.5.2", icon="database", name=("Base de données", "Database"),
  tag=("PostgreSQL, MySQL/MariaDB, SQLite et DuckDB : explorer, modifier, interroger.", "PostgreSQL, MySQL/MariaDB, SQLite and DuckDB: browse, edit, query."),
  intro=("Un client de bases de données intégré : l'arbre des sources dans la barre latérale, les tables éditables dans des onglets, une console SQL avec complétion, un concepteur de tables, des imports, exports et sauvegardes, et le diagramme ER.",
         "A database client built in: the tree of data sources in the sidebar, editable tables in tabs, a SQL console with completion, a table designer, imports, exports and backups, and the ER diagram."),
  groups=[
   (("Sources et arbre", "Sources and tree"), [
     ("PostgreSQL, MySQL / MariaDB, SQLite et DuckDB, en direct ou par un tunnel SSH, avec TLS.", "PostgreSQL, MySQL / MariaDB, SQLite and DuckDB, directly or through an SSH tunnel, with TLS."),
     ("Un fichier DuckDB n'est ouvert que par un programme à la fois : tant que le plugin l'a ouvert, un autre ne peut pas y écrire.", "A DuckDB file is opened by one program at a time: while the plugin has it open, another cannot write to it."),
     ("L'arbre de chaque source : bases, schémas, tables et leurs composants, fonctions, séquences, types ; sous l'arbre, les actions du nœud choisi (ouvrir, structure, modifier, DDL, import, copie, sauvegarde et restauration, vider, supprimer, nouvelle table ou schéma, diagramme ER, santé).", "The tree of each source: databases, schemas, tables and what they are made of, functions, sequences, types; under it, the actions for the node picked (open, structure, modify, DDL, import, copy, backup and restore, empty, drop, new table or schema, ER diagram, health)."),
     ("Mots de passe gardés dans le trousseau du système si vous le demandez.", "Passwords kept in the system keychain when asked to.")]),
   (("Une table dans un onglet", "A table in a tab"), [
     ("Les lignes par pages, une condition WHERE, une recherche, des filtres par colonne, des tris, des colonnes masquées, la ligne en formulaire.", "Rows by pages, a WHERE condition, a search, filters by column, sorts, hidden columns, the row as a form."),
     ("Cellules modifiées, lignes ajoutées et supprimées : tout attend le « Commit » (le SQL est montré d'abord) ; sélecteur de date ; liens vers les lignes référencées et celles qui référencent.", "Cells edited, rows added and deleted, all waiting for Commit (the SQL shown first); a date picker; links to the rows a row points to and those that point to it."),
     ("Export en CSV, JSON, SQL, XLSX et Markdown.", "Export to CSV, JSON, SQL, XLSX and Markdown.")]),
   (("Console SQL", "SQL console"), [
     ("Requêtes en onglets sous l'éditeur, complétion des tables et colonnes, exécution de l'instruction sous le curseur ou de la sélection, Explain, transactions.", "Queries in tabs under the editor, completion of tables and columns, the statement under the cursor or the selection run, Explain, transactions."),
     ("Valeurs demandées pour <code>:nom</code> et <code>?</code>, garde-fou avant une écriture sans WHERE, journal de ce qui a été exécuté.", "Values asked for <code>:name</code> and <code>?</code>, a guard before writes without WHERE, and the log of what was run.")]),
   (("Concevoir, comprendre, sauvegarder", "Design, understand, back up"), [
     ("Concepteur de tables : colonnes, clés et contraintes, index, nom et commentaire, et le SQL qui en résulte.", "Table designer: columns, keys and constraints, indexes, name and comment, and the SQL that comes of it."),
     ("Diagramme ER d'un schéma (copiable en Mermaid), santé d'une base (tailles, lignes mortes, index inutilisés).", "ER diagram of a schema (copied as Mermaid), health of a database (sizes, dead rows, unused indexes)."),
     ("Import CSV et JSON, copie d'une table vers une autre source, sauvegarde et restauration.", "Import of CSV and JSON, copy of a table to another source, backup and restore.")]),
  ],
  needs=[("Les pilotes (PostgreSQL, MySQL, SQLite, DuckDB) sont des programmes à part livrés avec le plugin : rien d'autre à installer.", "The drivers (PostgreSQL, MySQL, SQLite, DuckDB) are programs of their own, shipped with the plugin: nothing else to install.")],
  settings=("Le formulaire de connexion (tuile du type de source, six onglets, test de connexion) s'ouvre dans une fenêtre par-dessus l'éditeur.", "The connection form (a tile for the kind of source, six tabs, a connection test) opens in a window over the editor."),
  keys=[],
  media=[("plugin-database-tree.png", "Barre latérale avec une source PostgreSQL dépliée (schémas, tables) et une table ouverte dans un onglet.", "Sidebar with an expanded PostgreSQL source (schemas, tables) and a table open in a tab.", "img", "16/10"),
         ("plugin-database-console.png", "Console SQL sous l'éditeur : une requête, son résultat, la liste de complétion ouverte.", "SQL console under the editor: a query, its result, the completion list open.", "img", "16/10"),
         ("plugin-database-diagram.png", "Diagramme ER d'un schéma d'exemple.", "ER diagram of a sample schema.", "img", "16/10"),
         ("plugin-database-designer.png", "Concepteur de table ouvert (colonnes, index) avec le SQL généré.", "Table designer open (columns, indexes) with the generated SQL.", "img", "16/10"),
         ("plugin-database-demo.mp4", "Se connecter à une base (le Docker de test fait très bien l'affaire), ouvrir une table, modifier une cellule, voir le SQL puis Commit.", "Connect to a database (a test Docker container is perfect), open a table, edit a cell, see the SQL then Commit.", "video", "16/10")],
 ),
 dict(
  slug="api", repo="plugin-api", ver="1.3.2", icon="api", name=("API", "API"),
  tag=("Un Postman léger : requêtes, collections, environnements, tests.", "A light Postman: requests, collections, environments, tests."),
  intro=("Testez vos API HTTP sans quitter l'éditeur : requêtes dans des onglets, collections, environnements à variables, tests des réponses, et imports depuis Postman, OpenAPI ou cURL.",
         "Test your HTTP APIs without leaving the editor: requests in tabs, collections, environments with variables, tests of the answers, and imports from Postman, OpenAPI or cURL."),
  groups=[
   (("Requêtes", "Requests"), [
     ("Paramètres, en-têtes, corps JSON / texte / formulaire / multipart / fichier.", "Params, headers, JSON / text / form / multipart / file bodies."),
     ("Authentification Bearer, Basic ou clé d'API ; délai maximal, redirections, TLS.", "Bearer, Basic or API-key authentication; time limit, redirects, TLS.")]),
   (("Organiser", "Organise"), [
     ("Collections et dossiers, historique des requêtes.", "Collections and folders, request history."),
     ("Environnements avec des <code>{{variables}}</code> ; les valeurs secrètes sont gardées dans le trousseau.", "Environments with <code>{{variables}}</code>; secret values are kept in the keychain.")]),
   (("Tester", "Test"), [
     ("Tests sur les réponses et valeurs conservées d'une requête à l'autre ; exécution d'une collection entière.", "Tests of the answers and values kept from one request to the next; the run of a whole collection.")]),
   (("Importer, exporter", "Import, export"), [
     ("Imports et exports Postman, OpenAPI 3 / Swagger 2 et cURL.", "Imports and exports of Postman, OpenAPI 3 / Swagger 2 and cURL.")]),
  ],
  needs=[],
  settings=("Préférences › Plugins : certificats à approuver et proxy à utiliser.", "Preferences › Plugins: which certificates to trust and the proxy to go through."),
  keys=[],
  media=[("plugin-api-request.png", "Une requête POST JSON avec en-têtes, et la réponse (statut, temps, corps coloré) dessous.", "A JSON POST request with headers, and the response (status, time, highlighted body) below.", "img", "16/10"),
         ("plugin-api-collections.png", "Panneau des collections et environnements, avec une variable <code>{{base_url}}</code>.", "Collections and environments panel, with a <code>{{base_url}}</code> variable.", "img", "16/10"),
         ("plugin-api-tests.png", "Résultat des tests d'une collection exécutée.", "Test results of a collection run.", "img", "16/10")],
 ),
 dict(
  slug="lint", repo="plugin-lint", ver="1.0.2", icon="lint", name=("Lint", "Lint"),
  tag=("Les problèmes de vos fichiers, soulignés pendant la frappe, avec leurs corrections.", "The problems in your files, underlined as you type, with their fixes."),
  intro=("Le plugin lance les linters de votre projet et affiche leurs avertissements dans l'éditeur, avec leurs corrections. L'éditeur y ajoute ce que trouvent ses serveurs de langage.",
         "The plugin runs your project's linters and shows their findings in the editor, with their fixes. The editor adds what its language servers find."),
  groups=[
   (("Outils pris en charge", "Supported tools"), [
     ("ESLint, ruff, flake8, pycodestyle, pylint, mypy, Clippy, go vet, golangci-lint, cppcheck, ShellCheck, stylelint, yamllint, markdownlint, RuboCop, PHP, luacheck, ktlint, SwiftLint, SQLFluff, HLint, xmllint, et JSON lui-même.", "ESLint, ruff, flake8, pycodestyle, pylint, mypy, Clippy, go vet, golangci-lint, cppcheck, ShellCheck, stylelint, yamllint, markdownlint, RuboCop, PHP, luacheck, ktlint, SwiftLint, SQLFluff, HLint, xmllint, and JSON itself.")]),
   (("Dans l'éditeur", "In the editor"), [
     ("Problèmes affichés pendant que vous tapez, avec leur correction : « Rechercher les problèmes », « Corriger les problèmes du fichier », problème suivant / précédent.", "Problems shown as you type, with their fix: “Check for problems”, “Fix the problems of the file”, next / previous problem."),
     ("Les programmes propres au projet (<code>node_modules/.bin</code>, un virtualenv) ne tournent qu'une fois le dossier déclaré de confiance.", "A project's own programs (<code>node_modules/.bin</code>, a virtualenv) run only once the folder is trusted.")]),
  ],
  needs=[("Les linters eux-mêmes doivent être présents (dans le projet ou sur la machine).", "The linters themselves must be available (in the project or on the machine).")],
  settings=("Préférences › Lint : langages concernés, outils de chacun (ceux trouvés prêts dans le projet par défaut), chemin d'un programme introuvable.", "Preferences › Lint: the languages looked at, the tools of each one (those found ready in the project by default), and the path of a program that is not found by itself."),
  keys=[],
  media=[("plugin-lint-problems.png", "Un fichier avec des soulignements d'erreurs/avertissements et le menu des problèmes.", "A file with error/warning underlines and the Problems menu.", "img", "16/10"),
         ("plugin-lint-fix.png", "La correction proposée dans la bulle d'un problème.", "The fix offered in the card of a problem.", "img", "16/10")],
 ),
 dict(
  slug="spell", repo="plugin-spell", ver="1.0.2", icon="spell", name=("Orthographe", "Spelling"),
  tag=("Les mots mal orthographiés soulignés, avec suggestions et dictionnaire personnel.", "Misspelled words underlined, with suggestions and a personal dictionary."),
  intro=("Un correcteur orthographique discret basé sur les dictionnaires Hunspell de la machine : il souligne dans la prose (texte, Markdown hors code) et dans les commentaires des programmes.",
         "A quiet spell checker based on the machine's Hunspell dictionaries: it underlines in prose (text, Markdown outside code) and in the comments of programs."),
  groups=[
   (("Fonctionnement", "How it works"), [
     ("Souligne les mots inconnus des dictionnaires Hunspell, dans la prose et dans les commentaires du code.", "Underlines the words the Hunspell dictionaries do not know, in prose and in code comments."),
     ("<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>S</kbd> propose des suggestions pour le mot sous le curseur, l'ajoute au dictionnaire personnel ou l'ignore.", "<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>S</kbd> shows suggestions for the word at the cursor, adds it to the personal dictionary or ignores it.")]),
   (("Dictionnaires", "Dictionaries"), [
     ("Cherchés dans le dossier de données du plugin (<code>dictionaries/</code>), <code>~/.local/share/hunspell</code>, <code>/usr/share/hunspell</code>, <code>/usr/share/myspell</code>, ceux de Homebrew et <code>~/Library/Spelling</code> sur macOS.", "Looked for in the plugin's data folder (<code>dictionaries/</code>), <code>~/.local/share/hunspell</code>, <code>/usr/share/hunspell</code>, <code>/usr/share/myspell</code>, Homebrew's and <code>~/Library/Spelling</code> on macOS."),
     ("Un <code>.dic</code> avec son <code>.aff</code> : <code>apt install hunspell-fr</code> ou <code>brew install hunspell</code> les installent.", "A <code>.dic</code> with its <code>.aff</code>: <code>apt install hunspell-fr</code> or <code>brew install hunspell</code> leave them.")]),
  ],
  needs=[("Au moins un dictionnaire Hunspell (<code>hunspell-fr</code>, <code>hunspell-en-us</code>…).", "At least one Hunspell dictionary (<code>hunspell-fr</code>, <code>hunspell-en-us</code>…).")],
  settings=("La langue du dictionnaire se règle dans la page « Orthographe » des Préférences.", "The dictionary language is set on the Spelling page of the Preferences."),
  keys=[(("Ctrl", "Alt", "S"), "Suggestions pour le mot courant", "Suggestions for the current word")],
  media=[("plugin-spell-underline.png", "Un fichier Markdown avec quelques fautes soulignées et le menu de suggestions ouvert.", "A Markdown file with a few underlined mistakes and the suggestions menu open.", "img", "16/10")],
 ),
 dict(
  slug="history", repo="plugin-history", ver="1.1.2", icon="history", name=("Historique local", "Local History"),
  tag=("Ce qu'étaient vos fichiers, gardé sur la machine, que Git le sache ou non.", "What your files were, kept on this machine whether Git knows it or not."),
  intro=("Comme dans IntelliJ IDEA : chaque fichier est mémorisé à l'ouverture, à l'enregistrement, avant un remplacement dans les fichiers ou une suppression, et chaque minute pendant la frappe. Comparez, restaurez, posez des labels.",
         "As IntelliJ IDEA has it: a file is kept when it is opened, saved, rewritten by a replace in files, about to be deleted, and every minute while it is typed in. Compare, revert, put labels."),
  groups=[
   (("Panneau « Historique local : afficher »", "The “Local History: Show” panel"), [
     ("Ouvrez-le depuis la recherche d'actions et les menus de l'éditeur, d'un fichier et d'un dossier.", "Open it from the search for actions and the menus of the editor, of a file and of a folder."),
     ("Les révisions du fichier ou du dossier, les plus récentes d'abord, avec leurs labels ; choisissez-en une pour la comparer à l'état actuel, la remettre en place (ou une seule de ses modifications), ou l'enregistrer comme patch.", "The revisions of the file or folder, latest first, with the labels; pick one to compare it with now, put it back (or one of its changes), or save it as a patch.")]),
   (("Labels", "Labels"), [
     ("« Poser un label » marque un instant : revenir à un label remet en place tous les fichiers du dossier.", "“Put label” marks a moment: going back to a label puts back every file of the folder.")]),
   (("Stockage", "Storage"), [
     ("Les textes sont stockés une seule fois, par leur empreinte, dans le dossier de données du plugin.", "Texts are stored once, by their hash, in the plugin's data folder.")]),
  ],
  needs=[],
  settings=("Préférences › Plugins : combien de jours ouvrés sont conservés.", "Preferences › Plugins: how many working days are kept."),
  keys=[],
  media=[("plugin-history-panel.png", "Panneau d'historique d'un fichier : liste des révisions avec labels, diff avec l'état actuel.", "History panel of a file: list of revisions with labels, diff against the current state.", "img", "16/10")],
 ),
 dict(
  slug="tasks", repo="plugin-tasks", ver="1.0.3", icon="tasks", name=("Tâches", "Tasks"),
  tag=("Compiler, tester et lancer les commandes d'un projet, avec leur sortie.", "Build, test and run a project's commands, with their output."),
  intro=("Les commandes de build, de test et de lancement de votre projet (Cargo, scripts npm, Make, Go, ou votre propre fichier) s'exécutent depuis un panneau, avec leur sortie. Un emplacement cité dans la sortie s'ouvre au bon endroit.",
         "The build, test and run commands of your project (Cargo, npm scripts, Make, Go, or your own file) run from a panel, with their output. A place the output names opens the file there."),
  groups=[
   (("Détection automatique", "Automatic detection"), [
     ("Cargo, scripts npm, Make, Go, et le fichier <code>.ardoise/tasks.json</code> : <code>[{\"name\", \"command\", \"cwd\"}]</code>.", "Cargo, npm scripts, Make, Go, and the <code>.ardoise/tasks.json</code> file: <code>[{\"name\", \"command\", \"cwd\"}]</code>.")]),
   (("Sortie et erreurs", "Output and errors"), [
     ("Un emplacement cité par la sortie (<code>src/a.rs:12:5</code>) ouvre le fichier à cette ligne.", "A place the output names (<code>src/a.rs:12:5</code>) opens the file there."),
     ("Les erreurs et avertissements s'affichent dans leurs fichiers.", "Errors and warnings show in their files.")]),
   (("Sécurité", "Safety"), [
     ("Les commandes qu'écrit le projet lui-même ne s'exécutent qu'une fois que vous l'avez autorisé.", "The commands a project writes itself run once you allow it.")]),
  ],
  needs=[],
  settings=("Les tâches s'affichent dans un panneau ; <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>R</kbd> lance une tâche.", "Tasks show in a panel; <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>R</kbd> runs a task."),
  keys=[(("Ctrl", "Alt", "R"), "Lancer une tâche", "Run a task")],
  media=[("plugin-tasks-panel.png", "Panneau Tâches d'un projet Rust (build, test, run) avec une erreur de compilation cliquable.", "Tasks panel of a Rust project (build, test, run) with a clickable compile error.", "img", "16/10"),
         ("plugin-tasks-demo.mp4", "Lancer « test » puis cliquer sur une erreur : le fichier s'ouvre à la bonne ligne.", "Run “test” then click an error: the file opens at the right line.", "video", "16/10")],
 ),
 dict(
  slug="remote", repo="plugin-remote", ver="1.1.4", icon="remote", name=("Fichiers distants", "Remote Files"),
  tag=("Ouvrir et modifier les fichiers d'une autre machine par SSH.", "Open and edit the files of another machine over SSH."),
  intro=("C'est le <code>ssh</code> du système qui travaille : vos clés, votre agent et votre <code>~/.ssh/config</code> s'appliquent. Indiquez <code>user@hôte</code> et un dossier, choisissez un fichier : une copie est gardée ici, s'ouvre comme n'importe quel fichier, et repart à chaque enregistrement.",
         "The system's <code>ssh</code> does the work: keys, the agent and <code>~/.ssh/config</code> apply. Give <code>user@host</code> and a folder, pick a file: a copy is kept here and opened like any file, and sent back at each save."),
  groups=[
   (("Utilisation", "Usage"), [
     ("Donnez <code>user@hôte</code> et un dossier dans le panneau, puis choisissez un fichier.", "Give <code>user@host</code> and a folder in the panel, then pick a file."),
     ("<code>user@hôte:/chemin/fichier</code> ouvre directement ce fichier.", "<code>user@host:/path/file</code> opens that file at once."),
     ("À chaque enregistrement, le fichier repart sur la machine — sauf s'il y a changé entre-temps.", "At each save the file is sent back — unless it changed on the machine since.")]),
  ],
  needs=[("Un client <code>ssh</code> et un accès à la machine distante.", "An <code>ssh</code> client and access to the remote machine.")],
  settings=("Les machines récentes se vident depuis les Préférences (« Effacer »).", "Recent machines can be cleared from the Preferences (Clear)."),
  keys=[],
  media=[("plugin-remote-panel.png", "Panneau Fichiers distants : une machine connectée, un dossier déplié, un fichier ouvert dans un onglet.", "Remote Files panel: a connected machine, an expanded folder, a file open in a tab.", "img", "16/10")],
 ),
 dict(
  slug="terminal", repo="plugin-terminal", ver="1.1.1", icon="terminal", name=("Terminal", "Terminal"),
  tag=("Des shells en onglets sous l'éditeur, avec historique et recherche.", "Shells in tabs under the editor, with history and search."),
  intro=("Votre <code>$SHELL</code> dans un pseudo-terminal sous l'éditeur : xterm-256color, vraies couleurs, souris pour vim ou htop. Les chemins <code>fichier:ligne</code> s'ouvrent d'un Ctrl/Cmd+clic.",
         "Your <code>$SHELL</code> in a pseudo-terminal under the editor: xterm-256color, true color, the mouse for vim or htop. <code>file:line</code> paths open with a Ctrl/Cmd+click."),
  groups=[
   (("Fonctionnalités", "Features"), [
     ("Plusieurs shells en onglets ; historique avec recherche.", "Several shells in tabs; history with search."),
     ("<code>fichier:ligne</code> s'ouvre avec Cmd/Ctrl+clic dans l'éditeur.", "<code>file:line</code> opens with Cmd/Ctrl+click in the editor."),
     ("« Ouvrir dans le terminal » dans le menu d'un dossier ouvre un shell à cet endroit.", "“Open in Terminal” in the menu of a folder starts one there."),
     ("Copier / coller : <kbd>Ctrl</kbd>+<kbd>Maj</kbd>+<kbd>C</kbd> / <kbd>V</kbd> (<kbd>Cmd</kbd>+<kbd>C</kbd> / <kbd>V</kbd> sur macOS).", "Copy and paste: <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> / <kbd>V</kbd> (<kbd>Cmd</kbd>+<kbd>C</kbd> / <kbd>V</kbd> on macOS).")]),
  ],
  needs=[],
  settings=("Commandes : « Terminal : afficher », « Terminal : nouveau », « Ouvrir dans le terminal ».", "Commands: “Terminal: Show”, “Terminal: New”, “Open in Terminal”."),
  keys=[(("Alt", "F12"), "Afficher / masquer le terminal", "Show / hide the terminal")],
  media=[("plugin-terminal-panel.png", "Panneau terminal sous l'éditeur avec deux onglets de shell et une sortie colorée.", "Terminal panel under the editor with two shell tabs and colored output.", "img", "16/10"),
         ("plugin-terminal-demo.mp4", "Lancer une commande qui échoue, Ctrl+clic sur <code>fichier:ligne</code> : l'éditeur ouvre le fichier à la ligne.", "Run a command that fails, Ctrl+click on <code>file:line</code>: the editor opens the file at that line.", "video", "16/10")],
 ),
]
BY_SLUG = {p["slug"]: p for p in PLUGINS}
