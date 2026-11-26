# -*- coding: utf-8 -*-
from lib import *
from pages_home import ONELINER

RAW = "https://raw.githubusercontent.com/ardoise-editor/ardoise/main"


def build(out):
    tabs = [("auto", "Recommandé", "Recommended"), ("brew", "macOS · Homebrew", "macOS · Homebrew"),
            ("deb", "Debian · Ubuntu", "Debian · Ubuntu"), ("tar", "Linux · tar.gz", "Linux · tar.gz"),
            ("src", "Depuis les sources", "From source")]
    tb = "".join(f'<button type="button" role="tab" data-tab="{i}" aria-selected="false">{L(f, e)}</button>' for i, f, e in tabs)

    panels = f'''
<div class="tabpanel" id="auto" role="tabpanel">
 <p class="lead" style="font-size:17px">{L("Une seule commande pour Linux et macOS : elle détecte votre système, télécharge la dernière version et la vérifie (signature Ed25519 du fichier SHA256SUMS, puis somme de contrôle du fichier).", "One command for Linux and macOS: it detects your system, downloads the latest release and verifies it (Ed25519 signature on SHA256SUMS, then the file's checksum).")}</p>
 {cmd(ONELINER)}
 <h3 id="uninstall-auto" style="margin-top:28px">{L("Désinstaller", "Uninstall")}</h3>
 {cmd(f"curl -fsSL {RAW}/uninstall.sh | bash")}
 {P("Le script détecte comment Ardoise a été installé et le retire, avec tout ce qu'il avait conservé : réglages, session (le texte des fichiers non enregistrés), plugins et leurs données, serveurs de langage, et les mots de passe enregistrés dans le trousseau du système. Un dossier Scratch choisi par vous est laissé intact. Fermez Ardoise avant.", "The script detects how Ardoise was installed and removes it, with everything it kept: settings, session (the text of unsaved files), plugins and their data, language servers, and the passwords the plugins saved in the system keychain. A scratch folder you chose yourself is left alone. Close Ardoise first.")}
 {callout("pour garder vos données en vue d'une réinstallation, ajoutez <code>--keep-data</code> :", "to keep your data for a reinstall, add <code>--keep-data</code>:")}
 {cmd(f"curl -fsSL {RAW}/uninstall.sh | bash -s -- --keep-data")}
</div>

<div class="tabpanel" id="brew" role="tabpanel" hidden>
 <p class="lead" style="font-size:17px">{L("Installe <code>Ardoise.app</code> dans <code>/Applications</code> et place <code>ardoise</code> dans votre <code>PATH</code>.", "Installs <code>Ardoise.app</code> into <code>/Applications</code> and puts <code>ardoise</code> on your <code>PATH</code>.")}</p>
 {cmd("brew install --cask ardoise-editor/tap/ardoise")}
 {callout("l'application est signée « ad hoc », pas notarisée. Si Gatekeeper bloque le premier lancement, faites un clic droit sur l'app puis <b>Ouvrir</b>, ou lancez :", "the app is signed ad-hoc, not notarised. If Gatekeeper blocks the first launch, right-click the app and choose <b>Open</b>, or run:", label=("macOS", "macOS"), warn=True)}
 {cmd("xattr -dr com.apple.quarantine /Applications/Ardoise.app")}
 <h3 style="margin-top:28px">{L("Désinstaller", "Uninstall")}</h3>
 {cmd("brew uninstall --cask ardoise")}
 <p class="muted" style="margin-top:12px">{L("Pour supprimer aussi vos réglages :", "To drop your settings too:")}</p>
 {cmd("brew uninstall --zap --cask ardoise")}
</div>

<div class="tabpanel" id="deb" role="tabpanel" hidden>
 <p class="lead" style="font-size:17px">{L("Télécharge et installe le dernier paquet <code>.deb</code>.", "Downloads and installs the latest <code>.deb</code> package.")}</p>
 {code("""$ curl -fsSL https://api.github.com/repos/ardoise-editor/ardoise/releases/latest \\
  | sed -n 's/.*"browser_download_url": *"\\([^"]*\\/ardoise_[^"]*\\.deb\\)".*/\\1/p' \\
  | xargs curl -fsSLO
$ sudo apt install ./ardoise_*.deb && rm ardoise_*.deb""")}
 {callout("cette commande ne vérifie pas le téléchargement ; le script recommandé, lui, le fait. Pour contrôler à la main, téléchargez <code>SHA256SUMS</code> depuis la page des versions, à côté du <code>.deb</code>, puis :", "this command does not verify the download; the recommended script does. To check by hand, download <code>SHA256SUMS</code> from the Releases page next to the <code>.deb</code> and run:", label=("Vérification", "Verification"))}
 {cmd("sha256sum -c --ignore-missing SHA256SUMS")}
 <h3 style="margin-top:28px">{L("Désinstaller", "Uninstall")}</h3>
 {code("$ sudo apt remove ardoise\n# ou\n$ sudo dpkg -r ardoise")}
</div>

<div class="tabpanel" id="tar" role="tabpanel" hidden>
 <p class="lead" style="font-size:17px">{L("Pour n'importe quelle distribution Linux x86_64 : déposez le binaire dans <code>/usr/local/bin</code>.", "For any x86_64 Linux distribution: drop the binary into <code>/usr/local/bin</code>.")}</p>
 {code("""$ VERSION=$(curl -fsSL https://api.github.com/repos/ardoise-editor/ardoise/releases/latest | grep '"tag_name"' | head -1 | sed 's/.*"\\(.*\\)".*/\\1/')
$ curl -fsSL "https://github.com/ardoise-editor/ardoise/releases/download/${VERSION}/ardoise-${VERSION}-x86_64-linux.tar.gz" | tar xz
$ sudo mv ardoise /usr/local/bin/""")}
 <h3 style="margin-top:28px">{L("Désinstaller", "Uninstall")}</h3>
 {cmd("sudo rm /usr/local/bin/ardoise")}
</div>

<div class="tabpanel" id="src" role="tabpanel" hidden>
 <p class="lead" style="font-size:17px">{L("Il faut Rust 1.90 ou plus récent. Sous Ubuntu / Debian, installez d'abord les bibliothèques GTK 3 et libxkbcommon.", "You need Rust 1.90 or newer. On Ubuntu / Debian, install the GTK 3 and libxkbcommon development packages first.")}</p>
 {code("""# Prérequis (Ubuntu / Debian)
$ sudo apt-get install libgtk-3-dev libxkbcommon-dev

# Compiler et lancer
$ git clone https://github.com/ardoise-editor/ardoise
$ cd ardoise
$ cargo build --release
$ ./target/release/ardoise""")}
 <p class="muted" style="margin-top:12px">{L("Pour développer : <code>cargo run</code>, <code>cargo test</code>, <code>cargo clippy</code>, <code>cargo fmt</code>.", "To develop: <code>cargo run</code>, <code>cargo test</code>, <code>cargo clippy</code>, <code>cargo fmt</code>.")}</p>
</div>'''

    head = f'''<section class="page-head"><div class="wrap">
<div class="crumbs"><a href="index.html">{L("Accueil", "Home")}</a> / {L("Installation", "Install")}</div>
<h1>{L("Installer Ardoise", "Install Ardoise")}</h1>
<p class="lead">{L("Choisissez la méthode qui vous convient. Chacune a sa commande de désinstallation, et rien ne demande de compte.", "Pick the method that suits you. Each has its own uninstall command, and nothing asks for an account.")}</p></div></section>'''

    pick = f'''<section class="block tight" style="padding-top:24px"><div class="wrap" style="max-width:900px">
<div data-tabs data-detect="1" data-default="auto" data-mac="brew" data-linux="auto">
<div class="tabs" role="tablist" aria-label="{esc("Méthode / Method")}">{tb}</div>
{panels}
</div>
<p class="muted" style="margin-top:20px;font-size:14px">{L("Votre système est détecté pour pré-sélectionner l'onglet. Les versions sont sur la", "Your system is detected to pre-select the tab. Releases are on the")} <a href="{REPO}/releases/latest">{L("page des versions", "Releases page")}</a>.</p>
</div></section>'''

    req = table([("Système", "System"), ("Architecture", "Architecture"), ("Remarque", "Note")], [
        [("macOS", "macOS"), ("Apple Silicon (aarch64)", "Apple Silicon (aarch64)"), ("Signé ad hoc, non notarisé (voir l'onglet Homebrew).", "Signed ad-hoc, not notarised (see the Homebrew tab).")],
        [("Linux", "Linux"), ("x86_64", "x86_64"), ("Paquet .deb, tar.gz ou script ; bibliothèques GTK 3 et libxkbcommon pour compiler.", ".deb package, tar.gz or script; GTK 3 and libxkbcommon libraries to build.")],
    ])

    body = head + pick + f'''<section class="block tight"><div class="wrap">
<div class="doc" style="padding-top:0;grid-template-columns:1fr;gap:0"><div class="prose" style="max-width:900px">
{H(2, "Configuration requise", "Requirements", "req")}{req}
{H(2, "Premier lancement", "First launch", "first")}
{P("Au tout premier lancement, une fenêtre d'accueil règle l'essentiel : la langue, le thème, la taille du texte, l'enregistrement et le retour à la ligne, puis propose d'installer en un clic les plugins officiels. Elle ne s'affiche qu'une fois ; tout reste modifiable dans les Préférences.", "On the very first launch, a welcome window sets the essentials: language, theme, text size, saving and wrapping, then offers the official plugins to install in one click. It shows once; everything stays in the Preferences.")}
{slot("welcome.png", "La fenêtre d'accueil du premier lancement (« Bienvenue dans Ardoise ») avec la liste des plugins à installer. Pour la revoir : désinstaller avec le script, ou lancer avec un dossier de données vide.", "The first-launch welcome window with the list of plugins to install.", ratio="16/10", page="install.html", title="Bienvenue dans Ardoise")}
{H(2, "Lancer depuis le terminal", "Launch from the terminal", "cli")}
{code("""$ ardoise                  # ouvre la dernière session
$ ardoise chemin/fichier   # ouvre un fichier précis (plusieurs fichiers : un onglet chacun)
$ ardoise nouveau.txt      # un fichier qui n'existe pas s'ouvre vide ; enregistrer le crée""")}
{H(2, "Mettre à jour", "Update", "update")}
{P("Menu <b>Aide › Vérifier les mises à jour…</b> : Ardoise compare sa version à la dernière publiée, télécharge la mise à jour, en vérifie la somme de contrôle et la signature (elle est refusée si elle n'a pas été publiée par les mainteneurs), puis redémarre. Selon le mode d'installation, il se peut que la mise à jour intégrée ne soit pas disponible : téléchargez alors la nouvelle version à la main (ou <code>brew upgrade --cask ardoise</code>).", "Menu <b>Help › Check for updates…</b>: Ardoise compares its version to the latest one, downloads the update, verifies its checksum and signature (it is refused if it was not published by the maintainers), then restarts. Depending on how it was installed, the in-app update may be unavailable: download the new version by hand (or <code>brew upgrade --cask ardoise</code>).")}
{slot("update.png", "La fenêtre « Mise à jour » (« Vous êtes à jour ! » ou « La version X est disponible »).", "The Update window (“You're up to date!” or “Version X is available”).", ratio="16/10", page="install.html", title="Mise à jour")}
{H(2, "Où sont mes fichiers ?", "Where are my files?", "data")}
{table([("Quoi", "What"), ("Linux", "Linux"), ("macOS", "macOS")], [
    [("Session (y compris le texte non enregistré)", "Session (unsaved text included)"), "<code>~/.local/share/ardoise/session.json</code>", "<code>~/Library/Application Support/ardoise/session.json</code>"],
    [("Serveurs de langage", "Language servers"), "<code>~/.local/share/ardoise/lsp</code>", "<code>~/Library/Application Support/ardoise/lsp</code>"],
    [("Données des plugins", "Plugin data"), "<code>~/.local/share/ardoise/plugin-data/&lt;id&gt;/</code>", "<code>~/Library/Application Support/ardoise/plugin-data/&lt;id&gt;/</code>"],
])}
{callout("la session n'est lisible que par vous. Supprimez le dossier pour l'oublier. Les fichiers de plus de 64 Mo sont refusés ; ils doivent être en UTF-8, et les fins de ligne (LF / CRLF) et le retour final sont conservés tels quels.", "the session is readable by you only. Delete the folder to forget it. Files over 64 MB are refused; they must be UTF-8, and line endings (LF / CRLF) and the final newline are kept as they were.")}
{H(2, "Désinstaller", "Uninstall", "uninstall")}
{P("Chaque méthode d'installation a sa propre désinstallation, détaillée dans son onglet ci-dessus. Le résumé :", "Each install method has its own uninstall, detailed in its tab above. In short:")}
{table([("Installé avec", "Installed with"), ("Désinstaller avec", "Uninstall with")], [
    [("Script", "Script"), "<code>curl -fsSL …/uninstall.sh | bash</code> ( <code>--keep-data</code> )"],
    [("Homebrew", "Homebrew"), "<code>brew uninstall --cask ardoise</code> ( <code>--zap</code> )"],
    [(".deb", ".deb"), "<code>sudo apt remove ardoise</code>"],
    [("tar.gz", "tar.gz"), "<code>sudo rm /usr/local/bin/ardoise</code>"],
])}
</div></div></div></section>'''
    page("install.html", "install",
         ("Installation et désinstallation", "Install and uninstall"),
         ("Installer Ardoise Editor sur Linux ou macOS : script, Homebrew, .deb, tar.gz ou depuis les sources — et le désinstaller proprement.",
          "Install Ardoise Editor on Linux or macOS: script, Homebrew, .deb, tar.gz or from source — and uninstall it cleanly."),
         body, out)
