"""Validate release artifacts without installing or extracting their contents."""

import argparse
from email.parser import BytesParser
import os
from pathlib import Path
import tarfile
import zipfile

try:
    import tomllib as tomli
except ImportError:
    import tomli
from packaging.requirements import Requirement
from packaging.utils import (
    canonicalize_name,
    parse_sdist_filename,
    parse_wheel_filename,
)
from packaging.version import Version

ROOT = Path(__file__).resolve().parents[2]
PLATFORMS = {"linux-x86_64", "windows-x86_64", "macos-x86_64", "macos-arm64"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def check_config():
    def manifest(path):
        return tomli.loads((ROOT / path).read_text(encoding="utf-8"))

    core = manifest("core/Cargo.toml")["package"]["version"]
    wrapper = manifest("python/Cargo.toml")
    require(wrapper["package"]["version"] == core, "Core and wrapper versions differ")
    require(
        wrapper["dependencies"]["leankedin-park"]["version"] == "=" + core,
        "Wrapper must depend on the exact core version",
    )
    require(
        wrapper["package"]["publish"] is False, "Wrapper must not publish to crates.io"
    )
    require((ROOT / "Cargo.lock").is_file(), "Workspace Cargo.lock is missing")
    license_text = (ROOT / "LICENSE.md").read_bytes()
    for directory in ("core", "python"):
        require(
            (ROOT / directory / "LICENSE.md").read_bytes() == license_text,
            f"{directory}/LICENSE.md must match the root license",
        )
    if os.environ.get("MATURIN_VERSION"):
        requires = manifest("python/pyproject.toml")["build-system"]["requires"]
        maturin = [
            Requirement(value)
            for value in requires
            if canonicalize_name(Requirement(value).name) == "maturin"
        ]
        require(len(maturin) == 1, "Expected one Maturin build requirement")
        require(
            Version(os.environ["MATURIN_VERSION"]) in maturin[0].specifier,
            "CI Maturin version must satisfy the build backend requirement",
        )
    tag = os.environ.get("RELEASE_TAG")
    require(not tag or tag == "v" + core, f"Expected release tag v{core}, got {tag}")
    return Version(core), license_text


def check_metadata(data, version):
    metadata = BytesParser().parsebytes(data)
    require(
        canonicalize_name(metadata["Name"]) == "leankedin-park", "Wrong package name"
    )
    require(Version(metadata["Version"]) == version, "Wrong metadata version")
    require(metadata["Requires-Python"] == ">=3.10", "Unexpected Python requirement")


def wheel_platform(tags):
    require(
        all(t.interpreter == "cp310" and t.abi == "abi3" for t in tags),
        "Expected a cp310-abi3 wheel",
    )
    platforms = {t.platform for t in tags}
    if "manylinux_2_28_x86_64" in platforms:
        return "linux-x86_64"
    if platforms == {"win_amd64"}:
        return "windows-x86_64"
    for suffix, name in (("_x86_64", "macos-x86_64"), ("_arm64", "macos-arm64")):
        if all(p.startswith("macosx_") and p.endswith(suffix) for p in platforms):
            return name
    raise ValueError(f"Unexpected wheel platforms: {platforms}")


def check_wheel(path, version, license_text):
    name, actual_version, _, tags = parse_wheel_filename(path.name)
    require(
        name == "leankedin-park" and actual_version == version,
        "Wrong wheel name/version",
    )
    platform = wheel_platform(tags)
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        metadata = [n for n in names if n.endswith(".dist-info/METADATA")]
        require(len(metadata) == 1, "Expected exactly one wheel METADATA")
        check_metadata(archive.read(metadata[0]), version)
        require("leankedin_park/__init__.pyi" in names, "Wheel is missing type stubs")
        require("leankedin_park/py.typed" in names, "Wheel is missing py.typed")
        require(
            any(n.endswith((".so", ".pyd")) for n in names),
            "Wheel is missing its extension",
        )
        require(
            any(
                n.endswith("/LICENSE.md") and archive.read(n) == license_text
                for n in names
            ),
            "Wheel is missing the project license",
        )
    return platform


def check_sdist(path, version, license_text):
    name, actual_version = parse_sdist_filename(path.name)
    require(
        name == "leankedin-park" and actual_version == version,
        "Wrong sdist name/version",
    )
    with tarfile.open(path) as archive:
        # Strip only the distribution's top-level directory; do not extract files.
        members = {
            m.name.split("/", 1)[1]: m
            for m in archive.getmembers()
            if m.isfile() and "/" in m.name
        }
        for required in (
            "Cargo.lock",
            "core/Cargo.toml",
            "core/src/lib.rs",
            "python/Cargo.toml",
            "python/src/lib.rs",
            "PKG-INFO",
        ):
            require(required in members, f"Sdist is missing {required}")
        require(
            any(n.endswith("pyproject.toml") for n in members),
            "Sdist has no build backend",
        )
        require(
            any(n.endswith("leankedin_park.pyi") for n in members),
            "Sdist has no type stubs",
        )
        for name in ("core/LICENSE.md", "python/LICENSE.md"):
            require(name in members, f"Sdist is missing {name}")
            require(
                archive.extractfile(members[name]).read() == license_text,
                f"Incorrect license in {name}",
            )
        check_metadata(archive.extractfile(members["PKG-INFO"]).read(), version)


def check_distributions(directory, version, license_text):
    wheels = sorted(directory.glob("*.whl"))
    sdists = sorted(directory.glob("*.tar.gz"))
    require(
        len(wheels) == 4 and len(sdists) == 1,
        "Expected exactly four wheels and one sdist",
    )
    require(len(list(directory.iterdir())) == 5, "Unexpected extra release artifacts")
    platforms = {check_wheel(p, version, license_text) for p in wheels}
    require(platforms == PLATFORMS, f"Incomplete platform coverage: {platforms}")
    check_sdist(sdists[0], version, license_text)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config-only", action="store_true")
    parser.add_argument("--dist-dir", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    version, license_text = check_config()
    if not args.config_only:
        check_distributions(args.dist_dir, version, license_text)
    print(f"Package validation passed for {version}")
