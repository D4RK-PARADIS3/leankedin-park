"""Regression tests for the release gate, using small synthetic archives."""

import io
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile

from packaging.version import Version

from check_dist import check_distributions, check_sdist, check_wheel


class DistributionTests(unittest.TestCase):
    version = Version("0.1.0a1")
    license_text = b"Test project license\n"

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)

    def metadata(self, version="0.1.0a1"):
        return (
            f"Metadata-Version: 2.4\nName: leankedin-park\nVersion: {version}\n"
            "Requires-Python: >=3.10\n"
        ).encode()

    def wheel(
        self, platform="manylinux_2_28_x86_64", omit=None, version="0.1.0a1", abi="abi3"
    ):
        path = self.directory / f"leankedin_park-0.1.0a1-cp310-{abi}-{platform}.whl"
        files = {
            "leankedin_park/__init__.pyi": b"",
            "leankedin_park/py.typed": b"",
            "leankedin_park/leankedin_park.abi3.so": b"test fixture",
            "leankedin_park-0.1.0a1.dist-info/METADATA": self.metadata(version),
            "leankedin_park-0.1.0a1.dist-info/licenses/LICENSE.md": self.license_text,
        }
        with zipfile.ZipFile(path, "w") as archive:
            for name, data in files.items():
                if name != omit:
                    archive.writestr(name, data)
        return path

    def sdist(self, omit=None):
        path = self.directory / "leankedin_park-0.1.0a1.tar.gz"
        files = {
            n: b"test fixture"
            for n in (
                "Cargo.lock",
                "core/Cargo.toml",
                "core/src/lib.rs",
                "python/Cargo.toml",
                "python/src/lib.rs",
                "pyproject.toml",
                "python/leankedin_park.pyi",
            )
        }
        files.update(
            {
                "PKG-INFO": self.metadata(),
                "core/LICENSE.md": self.license_text,
                "python/LICENSE.md": self.license_text,
            }
        )
        with tarfile.open(path, "w:gz") as archive:
            for name, data in files.items():
                if name != omit:
                    member = tarfile.TarInfo("leankedin_park-0.1.0a1/" + name)
                    member.size = len(data)
                    archive.addfile(member, io.BytesIO(data))
        return path

    def test_complete_release(self):
        for platform in (
            "manylinux_2_28_x86_64",
            "win_amd64",
            "macosx_10_12_x86_64",
            "macosx_11_0_arm64",
        ):
            self.wheel(platform)
        self.sdist()
        check_distributions(self.directory, self.version, self.license_text)

    def test_missing_wheels(self):
        self.sdist()
        with self.assertRaisesRegex(ValueError, "four wheels"):
            check_distributions(self.directory, self.version, self.license_text)

    def test_missing_typing_files(self):
        for name in ("__init__.pyi", "py.typed"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "missing"):
                check_wheel(
                    self.wheel(omit="leankedin_park/" + name),
                    self.version,
                    self.license_text,
                )

    def test_metadata_version_mismatch(self):
        with self.assertRaisesRegex(ValueError, "metadata version"):
            check_wheel(self.wheel(version="9.0"), self.version, self.license_text)

    def test_wrong_license(self):
        with self.assertRaisesRegex(ValueError, "license"):
            check_wheel(self.wheel(), self.version, b"wrong license")

    def test_nonportable_wheel(self):
        with self.assertRaisesRegex(ValueError, "platforms"):
            check_wheel(
                self.wheel(platform="linux_x86_64"), self.version, self.license_text
            )

    def test_wrong_python_abi(self):
        with self.assertRaisesRegex(ValueError, "cp310-abi3"):
            check_wheel(self.wheel(abi="cp310"), self.version, self.license_text)

    def test_sdist_missing_local_dependency_or_lock(self):
        for name in ("core/src/lib.rs", "Cargo.lock"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "missing"):
                check_sdist(self.sdist(omit=name), self.version, self.license_text)


if __name__ == "__main__":
    unittest.main()
