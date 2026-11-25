"""The generator, built into a temporary folder, and the pages published in docs/."""
import glob, os, re, shutil, subprocess, sys, tempfile, unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DOCS = os.path.join(ROOT, "docs")
# The app draws the ER diagram without any text: its two screenshots are still to come.
PENDING = {"media/plugin-database-diagram.png", "media/plugin-database-diagram-en.png"}


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


class Build(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp()
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        subprocess.run([sys.executable, os.path.join(ROOT, "site-generator", "build.py"), "--repo", ROOT, "--out", cls.tmp],
                       check=True, capture_output=True, env=env)

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp)

    def pages(self):
        return glob.glob(os.path.join(self.tmp, "*.html"))

    def test_docs_is_up_to_date(self):
        for path in self.pages() + glob.glob(os.path.join(self.tmp, "assets", "*")):
            rel = os.path.relpath(path, self.tmp)
            with open(path, "rb") as a, open(os.path.join(DOCS, rel), "rb") as b:
                self.assertEqual(a.read(), b.read(), rel + " differs: run build.py")

    def test_plugin_guide_menu_added_once(self):
        t = read(os.path.join(self.tmp, "plugins.html"))
        self.assertEqual(t.count('href="extensions.html"'), 1)

    def test_every_media_has_both_languages(self):
        slots = 0
        for page in self.pages():
            for figure in re.findall(r'<figure class="slot".*?</figure>', read(page), re.S):
                slots += 1
                found = dict(re.findall(r'<(?:img|video) data-lang="(fr|en)".*?src="(media/[^"]+\.(?:png|mp4))"', figure, re.S))
                self.assertEqual(set(found), {"fr", "en"}, page)
                base, ext = found["fr"].rsplit(".", 1)
                self.assertEqual(found["en"], f"{base}-en.{ext}", page)
        self.assertGreater(slots, 40)

    def test_hero_has_both_languages(self):
        t = read(os.path.join(self.tmp, "index.html"))
        self.assertIn('data-lang="fr" src="media/hero.png"', t)
        self.assertIn('data-lang="en" src="media/hero-en.png"', t)


class Published(unittest.TestCase):
    def test_links_and_media_exist(self):
        missing = set()
        for page in glob.glob(os.path.join(DOCS, "*.html")):
            for url in re.findall(r'(?:href|src|poster)="([^"]+)"', read(page)):
                if re.match(r"(?:https?:|mailto:|data:|#|/)", url):
                    continue
                path = url.split("#")[0].split("?")[0]
                if path and not os.path.exists(os.path.join(DOCS, path)):
                    missing.add(path)
        self.assertEqual(missing - PENDING, set())


if __name__ == "__main__":
    unittest.main()
