#!/usr/bin/env python3
"""
Generate sparse cache indexes for mathlib.

This script scans a mathlib checkout and its compiled `.olean` files to produce
an index JSON file compatible with Lemma's sparse cache downloader.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import platform
import subprocess
import sys
from collections import OrderedDict
from pathlib import Path
from typing import Dict, List, Tuple

_IMPORT_PREFIX = "Mathlib."


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate sparse cache index for mathlib"
    )
    parser.add_argument(
        "--mathlib-root",
        type=Path,
        default=Path("mathlib"),
        help="Path to the mathlib checkout (default: ./mathlib)",
    )
    parser.add_argument(
        "--olean-root",
        type=Path,
        default=Path(".lake") / "build" / "lib",
        help="Path to the directory that contains Mathlib .olean files "
        "(default: .lake/build/lib)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("index.json"),
        help="Where to write the generated index (default: ./index.json)",
    )
    parser.add_argument(
        "--platform",
        help="Platform identifier (defaults to host platform, e.g. linux-x86_64)",
    )
    parser.add_argument(
        "--lean-version",
        help="Lean toolchain identifier (defaults to contents of lean-toolchain file)",
    )
    parser.add_argument(
        "--mathlib-commit",
        help="Mathlib commit hash (defaults to `git rev-parse HEAD` in mathlib root)",
    )
    parser.add_argument(
        "--allow-missing",
        action="store_true",
        help="Skip modules whose .olean file is missing instead of failing",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Enable verbose logging",
    )
    return parser.parse_args()


def log(message: str, *, verbose: bool = True) -> None:
    if verbose:
        print(f"[sparse-index] {message}")


def detect_platform() -> str:
    system = platform.system().lower()
    arch = platform.machine().lower()

    system_map = {
        "linux": "linux",
        "darwin": "macos",
        "windows": "windows",
    }
    arch_map = {
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "arm64": "aarch64",
        "aarch64": "aarch64",
    }

    normalized_system = system_map.get(system, system)
    normalized_arch = arch_map.get(arch, arch)
    return f"{normalized_system}-{normalized_arch}"


def git_commit(root: Path) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return result.stdout.strip()


def read_lean_toolchain(root: Path) -> str:
    toolchain_file = root / "lean-toolchain"
    if not toolchain_file.exists():
        raise FileNotFoundError(f"lean-toolchain not found in {root}")
    return toolchain_file.read_text(encoding="utf-8").strip()


def sha256sum(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_imports(lean_path: Path) -> List[str]:
    """Parse import statements from a Lean file."""
    imports: List[str] = []
    seen = set()
    with lean_path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.split("--", 1)[0].strip()
            if not line or not line.startswith("import"):
                continue
            modules_part = line[len("import") :].strip()
            if not modules_part:
                continue
            for raw_module in modules_part.split():
                normalized = normalize_module_name(raw_module)
                if normalized and normalized not in seen:
                    seen.add(normalized)
                    imports.append(normalized)
    return imports


def normalize_module_name(module: str) -> str:
    module = module.strip()
    if not module:
        return ""
    if module.startswith(_IMPORT_PREFIX):
        return module
    if module.startswith("Init."):
        # Skip core Lean modules because they are not hosted in mathlib cache
        return ""
    if (
        module.startswith("Aesop")
        or module.startswith("Batteries")
        or module.startswith("Qq")
        or module.startswith("Plausible")
        or module.startswith("ImportGraph")
        or module.startswith("LeanSearchClient")
        or module.startswith("ProofWidgets")
    ):
        # Skip external dependencies
        return ""
    if (
        module.startswith("Lake.")
        or module.startswith("Lean.")
        or module.startswith("Std.")
    ):
        return ""
    return f"{_IMPORT_PREFIX}{module}"


def collect_modules(
    mathlib_root: Path, olean_root: Path, *, allow_missing: bool, verbose: bool
) -> Tuple[Dict[str, dict], List[str]]:
    modules: Dict[str, dict] = {}
    missing: List[str] = []
    mathlib_dir = mathlib_root / "Mathlib"
    if not mathlib_dir.is_dir():
        raise FileNotFoundError(
            f"{mathlib_dir} does not exist. Did you pass the mathlib root?"
        )

    for lean_path in sorted(mathlib_dir.rglob("*.lean")):
        rel = lean_path.relative_to(mathlib_root)
        module_name = ".".join(rel.with_suffix("").parts)
        olean_rel = rel.with_suffix(".olean")
        olean_path = olean_root / olean_rel

        if not olean_path.exists():
            missing.append(module_name)
            if allow_missing:
                log(f"Skipping {module_name} (missing {olean_path})", verbose=verbose)
                continue
            raise FileNotFoundError(
                f"Missing .olean for module {module_name}: {olean_path}"
            )

        dependencies = [
            dep for dep in parse_imports(lean_path) if dep.startswith(_IMPORT_PREFIX)
        ]

        modules[module_name] = {
            "path": str(Path(*olean_rel.parts)),
            "size": olean_path.stat().st_size,
            "sha256": sha256sum(olean_path),
            "dependencies": dependencies,
        }

    return modules, missing


def main() -> int:
    args = parse_args()

    mathlib_root = args.mathlib_root.resolve()
    olean_root = args.olean_root.resolve()

    if not mathlib_root.exists():
        print(f"Mathlib root {mathlib_root} does not exist", file=sys.stderr)
        return 1
    if not olean_root.exists():
        print(f"Olean root {olean_root} does not exist", file=sys.stderr)
        return 1

    lean_version = args.lean_version or read_lean_toolchain(mathlib_root)
    mathlib_commit = args.mathlib_commit or git_commit(mathlib_root)
    platform_id = args.platform or detect_platform()

    log(f"Using mathlib root: {mathlib_root}", verbose=args.verbose)
    log(f"Using olean root: {olean_root}", verbose=args.verbose)
    log(f"Detected Lean toolchain: {lean_version}", verbose=args.verbose)
    log(f"Detected mathlib commit: {mathlib_commit}", verbose=args.verbose)
    log(f"Detected platform: {platform_id}", verbose=args.verbose)

    modules, missing = collect_modules(
        mathlib_root,
        olean_root,
        allow_missing=args.allow_missing,
        verbose=args.verbose,
    )

    created_at = dt.datetime.utcnow().replace(microsecond=0).isoformat() + "Z"

    index = OrderedDict(
        version=1,
        lean_version=lean_version,
        mathlib_commit=mathlib_commit,
        platform=platform_id,
        created_at=created_at,
        modules=modules,
    )

    output_path = args.output.resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as handle:
        json.dump(index, handle, indent=2)

    log(f"Wrote index with {len(modules)} modules to {output_path}", verbose=True)

    if missing and args.verbose:
        log(f"{len(missing)} modules were missing .olean files", verbose=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
