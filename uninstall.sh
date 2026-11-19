#!/usr/bin/env bash
set -euo pipefail

# ── helpers ──────────────────────────────────────────────────────────────────
info()  { echo "  $*"; }
ok()    { echo "✓ $*"; }
err()   { echo "✗ $*" >&2; exit 1; }

REMOVED=0
REFRESH_SYS=0
REFRESH_USER=0
# Everything goes by default: settings, session, plugins and their data,
# language servers, and the passwords kept in the system keychain.
# `--keep-data` (or ARDOISE_KEEP_DATA=1) keeps them for a reinstall.
KEEP_DATA="${ARDOISE_KEEP_DATA:-${SIMPLEEDIT_KEEP_DATA:-0}}"
for arg in "$@"; do
    case "$arg" in
        --keep-data) KEEP_DATA=1 ;;
        *) err "Unknown option: $arg" ;;
    esac
done
# The name the plugins keep their passwords under in the keychain (it kept its
# first name, SimpleEdit, so as not to lose what is saved there).
KEYCHAIN_SERVICE="SimpleEdit data source"

# ── platform detection ───────────────────────────────────────────────────────
OS=$(uname -s)

# A running Ardoise writes its settings and session again as it quits, and
# may hold unsaved work: it is closed first, by the user.
if command -v pgrep &>/dev/null && { pgrep -x ardoise || pgrep -x simpleedit; } &>/dev/null; then
    err "Ardoise is running. Close it (and save your work), then run this again."
fi

case "$OS" in
    Linux)
        # .deb / apt package (also catches the leftover "config-files" state
        # left by older versions of this script that used `remove`, not `purge`)
        # (the package was called simpleedit before the app was renamed)
        for pkg in ardoise simpleedit; do
            if command -v dpkg &>/dev/null && dpkg -s "$pkg" &>/dev/null; then
                info "Removing .deb package ${pkg} (requires sudo)…"
                sudo apt-get purge -y "$pkg" 2>/dev/null || sudo dpkg -P "$pkg"
                REMOVED=1
                REFRESH_SYS=1
            fi
        done

        # snap package
        for pkg in ardoise simpleedit; do
            if command -v snap &>/dev/null && snap list "$pkg" &>/dev/null; then
                info "Removing snap package ${pkg}…"
                sudo snap remove "$pkg"
                REMOVED=1
            fi
        done

        # Stray binaries. Both dirs sit ahead of /usr/bin on a typical PATH, so
        # a leftover here silently shadows the packaged binary after a reinstall.
        for name in ardoise simpleedit; do
            if [ -e "/usr/local/bin/$name" ] || [ -L "/usr/local/bin/$name" ]; then
                info "Removing /usr/local/bin/${name} (requires sudo)…"
                sudo rm -f "/usr/local/bin/$name"
                REMOVED=1
            fi
            if [ -e "$HOME/.local/bin/$name" ] || [ -L "$HOME/.local/bin/$name" ]; then
                info "Removing ~/.local/bin/${name}…"
                rm -f "$HOME/.local/bin/$name"
                REMOVED=1
            fi
        done

        # AppImage and its desktop integration
        for f in "$HOME/.local/bin/ardoise.AppImage" \
                 "$HOME/Applications/ardoise.AppImage" \
                 "$HOME/.local/share/applications/ardoise.desktop" \
                 "$HOME/.local/bin/simpleedit.AppImage" \
                 "$HOME/Applications/simpleedit.AppImage" \
                 "$HOME/.local/share/applications/simpleedit.desktop"; do
            if [ -e "$f" ]; then
                info "Removing ${f/#$HOME/\~}…"
                rm -f "$f"
                REMOVED=1
                REFRESH_USER=1
            fi
        done


        # Refresh menu / icon caches so a stale launcher entry disappears now
        if [ "$REFRESH_SYS" -eq 1 ]; then
            sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
            sudo update-desktop-database /usr/share/applications 2>/dev/null || true
        fi
        if [ "$REFRESH_USER" -eq 1 ]; then
            update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
        fi

        # (the folders were called simpleedit before the app was renamed)
        DATA_DIRS=("${XDG_CONFIG_HOME:-$HOME/.config}/ardoise" "${XDG_DATA_HOME:-$HOME/.local/share}/ardoise"
                   "${XDG_CONFIG_HOME:-$HOME/.config}/simpleedit" "${XDG_DATA_HOME:-$HOME/.local/share}/simpleedit")
        ;;

    Darwin)
        # (the cask was called simpleedit before the app was renamed)
        for cask in ardoise simpleedit; do
            if command -v brew &>/dev/null && brew list --cask "$cask" &>/dev/null; then
                info "Uninstalling the Homebrew cask ${cask}…"
                brew uninstall --cask "$cask"
                REMOVED=1
            fi
        done
        if command -v brew &>/dev/null; then
            brew untap ardoise-editor/tap &>/dev/null || true
            brew untap simple-edit-app/tap &>/dev/null || true
        fi
        for app in Ardoise SimpleEdit; do
            if [ -d "/Applications/${app}.app" ]; then
                info "Removing /Applications/${app}.app…"
                # An app installed by an administrator needs sudo; try without first.
                rm -rf "/Applications/${app}.app" 2>/dev/null || sudo rm -rf "/Applications/${app}.app"
                REMOVED=1
            fi
        done
        for name in ardoise simpleedit; do
            if [ -e "/usr/local/bin/$name" ] || [ -L "/usr/local/bin/$name" ]; then
                info "Removing binary /usr/local/bin/${name} (requires sudo)…"
                sudo rm -f "/usr/local/bin/$name"
                REMOVED=1
            fi
            if [ -e "$HOME/.local/bin/$name" ] || [ -L "$HOME/.local/bin/$name" ]; then
                info "Removing binary ~/.local/bin/${name}…"
                rm -f "$HOME/.local/bin/$name"
                REMOVED=1
            fi
        done

        DATA_DIRS=("$HOME/Library/Application Support/ardoise"
                   "$HOME/Library/Application Support/simpleedit"
                   "$HOME/Library/Saved Application State/app.ardoise.Ardoise.savedState"
                   "$HOME/Library/Saved Application State/app.simpleedit.SimpleEdit.savedState"
                   "$HOME/Library/Preferences/app.ardoise.Ardoise.plist"
                   "$HOME/Library/Preferences/app.simpleedit.SimpleEdit.plist"
                   "$HOME/Library/Caches/app.ardoise.Ardoise"
                   "$HOME/Library/Caches/app.simpleedit.SimpleEdit")
        ;;

    *)
        err "Unsupported OS: $OS"
        ;;
esac

# ── the data ─────────────────────────────────────────────────────────────────

# The passwords the plugins kept in the system keychain (data sources, API
# credentials). Returns 1 when they could not be removed.
forget_keychain() {
    case "$OS" in
        Darwin)
            while security delete-generic-password -s "$KEYCHAIN_SERVICE" &>/dev/null; do :; done
            return 0
            ;;
        Linux)
            if command -v secret-tool &>/dev/null; then
                secret-tool clear service "$KEYCHAIN_SERVICE" 2>/dev/null || true
                return 0
            fi
            # Without secret-tool, the Secret Service over D-Bus (GNOME
            # Keyring, KWallet): each item found under that service is deleted.
            if command -v gdbus &>/dev/null; then
                local found items item
                found=$(gdbus call --session --dest org.freedesktop.secrets \
                    --object-path /org/freedesktop/secrets \
                    --method org.freedesktop.Secret.Service.SearchItems \
                    "{'service': '$KEYCHAIN_SERVICE'}" 2>/dev/null) || return 1
                items=$(printf '%s' "$found" | grep -o "/org/freedesktop/secrets/[^']*" || true)
                for item in $items; do
                    gdbus call --session --dest org.freedesktop.secrets \
                        --object-path "$item" \
                        --method org.freedesktop.Secret.Item.Delete &>/dev/null || return 1
                done
                return 0
            fi
            return 1
            ;;
    esac
}

# Nothing of the app, nothing of its data: there is nothing to do.
HAS_DATA=0
for dir in "${DATA_DIRS[@]}"; do
    [ -e "$dir" ] && HAS_DATA=1
done
if [ "$REMOVED" -eq 0 ] && { [ "$HAS_DATA" -eq 0 ] || [ "$KEEP_DATA" = "1" ]; }; then
    err "Ardoise does not appear to be installed."
fi
[ "$REMOVED" -eq 1 ] || info "The app itself was already removed: its data goes now."

if [ "$KEEP_DATA" = "1" ]; then
    ok "Ardoise uninstalled."
    info "Your settings, plugins and their data were kept (--keep-data)."
    exit 0
fi

# A folder of scratch files chosen by the user is theirs: it stays.
SCRATCH=""
for dir in "${DATA_DIRS[@]}"; do
    if [ -f "$dir/config.json" ]; then
        SCRATCH=$(sed -n 's/.*"scratch_dir": *"\([^"]*\)".*/\1/p' "$dir/config.json" | head -n 1)
    fi
done

if forget_keychain; then
    ok "Passwords removed from the system keychain"
else
    info "The passwords saved by the plugins could not be removed from the keychain"
    info "(no secret-tool): look for \"$KEYCHAIN_SERVICE\" in Passwords and Keys."
fi

for dir in "${DATA_DIRS[@]}"; do
    if [ -e "$dir" ]; then
        info "Removing ${dir/#$HOME/\~}…"
        rm -rf "$dir"
    fi
done

# ── done ─────────────────────────────────────────────────────────────────────
ok "Ardoise uninstalled, with its settings, plugins and their data."
if [ -n "$SCRATCH" ] && [ -d "$SCRATCH" ]; then
    info "Your scratch files in ${SCRATCH/#$HOME/\~} were kept: they are yours."
fi
