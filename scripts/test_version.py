"""Exercise release version commands against an isolated Cargo workspace."""

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("version.py")
INITIAL_VERSION = "0.2.0-alpha"


class VersionCommands(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="derse-version-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "scripts").mkdir()
        shutil.copyfile(SCRIPT, self.root / "scripts/version.py")
        self.write(
            "Cargo.toml",
            '[workspace]\n'
            'members = ["derse", "derse-derive", "tests/renamed"]\n'
            'resolver = "2"\n'
            '\n[workspace.package]\n'
            'version = "0.2.0-alpha"\n'
            'edition = "2021"\n'
            '\n[workspace.dependencies]\n'
            'derse-derive = { version = "=0.2.0-alpha", path = "derse-derive" }\n',
        )
        self.write("README.md", '# Derse\n\n```toml\nderse = "=0.2.0-alpha"\n```\n')
        self.write(
            "derse/Cargo.toml",
            '[package]\nname = "derse"\nversion.workspace = true\n'
            'edition.workspace = true\n'
            '\n[dependencies]\nderse-derive.workspace = true\n',
        )
        self.write(
            "derse-derive/Cargo.toml",
            '[package]\nname = "derse-derive"\nversion.workspace = true\n'
            'edition.workspace = true\n'
            '\n[lib]\nproc-macro = true\n'
            '\n[dev-dependencies]\nderse = { path = "../derse" }\n',
        )
        self.write(
            "tests/renamed/Cargo.toml",
            '[package]\nname = "derse-renamed-tests"\nversion.workspace = true\n'
            'edition.workspace = true\npublish = false\n'
            '\n[dependencies]\nds = { package = "derse", path = "../../derse" }\n',
        )
        for member in ("derse", "derse-derive", "tests/renamed"):
            self.write(member + "/src/lib.rs", "")

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def replace(self, relative, before, after):
        path = self.root / relative
        content = path.read_text()
        self.assertIn(before, content)
        path.write_text(content.replace(before, after))

    def run_command(self, *arguments, success=True):
        result = subprocess.run(
            [sys.executable, str(self.root / "scripts/version.py"), *arguments],
            cwd=self.root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        output = result.stdout + result.stderr
        if success:
            self.assertEqual(result.returncode, 0, output)
        else:
            self.assertNotEqual(result.returncode, 0, output)
            self.assertIn("version:", result.stderr)
        return result

    def release_files(self):
        paths = list(self.root.rglob("Cargo.toml")) + [self.root / "README.md"]
        return {path.relative_to(self.root): path.read_bytes() for path in paths}

    def test_check_accepts_synchronized_workspace_and_private_fixture(self):
        result = self.run_command("check")
        self.assertIn(INITIAL_VERSION, result.stdout)
        self.assertIn("publish: derse, derse-derive", result.stdout)
        self.assertNotIn("derse-renamed-tests", result.stdout)

    def test_check_rejects_member_versions_that_do_not_inherit(self):
        for version in (INITIAL_VERSION, "0.2.0"):
            with self.subTest(version=version):
                manifest = self.root / "tests/renamed/Cargo.toml"
                original = manifest.read_text()
                manifest.write_text(
                    original.replace("version.workspace = true", 'version = "' + version + '"')
                )
                result = self.run_command("check", success=False)
                self.assertIn("must inherit", result.stderr)
                manifest.write_text(original)

    def test_check_rejects_relaxed_internal_dependency(self):
        self.replace("Cargo.toml", 'version = "=0.2.0-alpha"', 'version = "0.2.0-alpha"')
        result = self.run_command("check", success=False)
        self.assertIn("pin internal dependencies", result.stderr)

    def test_check_rejects_missing_runtime_derive_dependency(self):
        self.replace("derse/Cargo.toml", "derse-derive.workspace = true", "")
        result = self.run_command("check", success=False)
        self.assertIn("derse must depend", result.stderr)

    def test_check_rejects_registry_internal_dependency(self):
        self.replace("Cargo.toml", ', path = "derse-derive"', "")
        result = self.run_command("check", success=False)
        self.assertIn("local workspace dependencies", result.stderr)

    def test_check_rejects_versioned_runtime_dev_dependency(self):
        manifest = self.root / "derse-derive/Cargo.toml"
        original = manifest.read_text()
        for requirement in ("=0.2.0-alpha", "*"):
            with self.subTest(requirement=requirement):
                manifest.write_text(
                    original.replace(
                        'derse = { path = "../derse" }',
                        'derse = { version = "' + requirement + '", path = "../derse" }',
                    )
                )
                result = self.run_command("check", success=False)
                self.assertIn("derive dev-dependency", result.stderr)
                manifest.write_text(original)

    def test_check_rejects_readme_version_drift(self):
        self.replace("README.md", INITIAL_VERSION, "0.2.0")
        result = self.run_command("check", success=False)
        self.assertIn("README", result.stderr)

    def test_check_rejects_publishable_test_fixture(self):
        self.replace("tests/renamed/Cargo.toml", "publish = false", "publish = true")
        result = self.run_command("check", success=False)
        self.assertIn("only derse and derse-derive may be published", result.stderr)

    def test_set_synchronizes_prerelease_and_stable_versions(self):
        for version in ("0.2.0-alpha.1", "0.2.0"):
            with self.subTest(version=version):
                self.run_command("set", version)
                metadata = subprocess.check_output(
                    ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"],
                    cwd=self.root,
                    text=True,
                )
                packages = json.loads(metadata)["packages"]
                self.assertEqual({package["version"] for package in packages}, {version})
                runtime = next(package for package in packages if package["name"] == "derse")
                derive = next(
                    dep for dep in runtime["dependencies"] if dep["name"] == "derse-derive"
                )
                self.assertEqual(derive["req"], "=" + version)
                self.assertIn('derse = "=' + version + '"', (self.root / "README.md").read_text())
                self.run_command("check")

    def test_set_rejects_invalid_versions_without_writing_files(self):
        original = self.release_files()
        for version in (
            "0.2", "v0.2.0", "00.2.0", "0.2.0-", "0.2.0-alpha..1", "0.2.0-01", "0.2.0+build"
        ):
            with self.subTest(version=version):
                self.run_command("set", version, success=False)
                self.assertEqual(self.release_files(), original)

    def test_set_rolls_back_when_release_validation_fails(self):
        self.replace("tests/renamed/Cargo.toml", "publish = false", "publish = true")
        original = self.release_files()
        self.run_command("set", "0.2.0-alpha.1", success=False)
        self.assertEqual(self.release_files(), original)


if __name__ == "__main__":
    unittest.main()
