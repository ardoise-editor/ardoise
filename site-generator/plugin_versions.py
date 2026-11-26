# -*- coding: utf-8 -*-
"""The versions of the plugins shown on the site: those of their latest releases.

`plugin_versions.json` holds them, so that a build gives the same pages anywhere.
Running this file brings it up to date; the deployment of the site does it each
time, and once a day.

Usage:  python3 site-generator/plugin_versions.py
"""
import json
import os
import re
import urllib.request

FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "plugin_versions.json")
LATEST = "https://api.github.com/repos/ardoise-editor/%s/releases/latest"


def load():
    with open(FILE, encoding="utf-8") as f:
        return json.load(f)


def latest_tag(repo):
    """The tag of the latest release of a repository. A `GITHUB_TOKEN`, if any,
    only raises the number of requests GitHub allows."""
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "site-generator"}
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = "Bearer " + token
    with urllib.request.urlopen(urllib.request.Request(LATEST % repo, headers=headers), timeout=20) as answer:
        return json.load(answer).get("tag_name")


def refresh(versions, latest=latest_tag):
    """The versions of the latest releases. A plugin GitHub cannot tell about
    keeps the one it had: the site is published all the same."""
    found = {}
    for slug, version in versions.items():
        found[slug] = version
        try:
            tag = latest("plugin-" + slug)
        except Exception as e:  # network, rate limit, unreadable answer
            print("::warning::%s: %s, %s kept" % (slug, e, version))
            continue
        match = re.fullmatch(r"v(\d+\.\d+\.\d+)", tag or "")
        if match:
            found[slug] = match.group(1)
        else:
            print("::warning::%s: unexpected tag %r, %s kept" % (slug, tag, version))
    return found


def main():
    before = load()
    after = refresh(before)
    for slug, version in after.items():
        if version != before[slug]:
            print("%s: %s -> %s" % (slug, before[slug], version))
    with open(FILE, "w", encoding="utf-8") as f:
        json.dump(after, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
