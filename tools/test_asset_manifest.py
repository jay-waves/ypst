from pathlib import Path
import tomllib
import unittest


ROOT = Path(__file__).resolve().parent.parent


class AssetManifestTests(unittest.TestCase):
    def test_checkout_inventory(self):
        with (ROOT / "typst.toml").open("rb") as manifest:
            data = tomllib.load(manifest)
        self.assertEqual(data["package"]["version"], "0.1.0")
        assets = data["tool"]["ypst-template"]["assets"]
        for name, asset in assets.items():
            for file in asset.get("files", []):
                self.assertTrue((ROOT / file).is_file(), (name, file))
            if "license" in asset:
                self.assertTrue((ROOT / asset["license"]).is_file(), name)



if __name__ == "__main__":
    unittest.main()
