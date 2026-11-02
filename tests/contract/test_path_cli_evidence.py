"""Check CLI subprocess evidence, historical source hashes and Cargo inventory."""
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "tests/evidence/path-cli-2026-11-03"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def observed_input(path, expected):
    current = (ROOT / path).read_bytes()
    if sha(current) == expected:
        return current
    # When source changes later, recover the source from the commit adding this
    # immutable receipt. A shallow clone missing that object fails explicitly.
    commit = subprocess.check_output([
        "git", "log", "-1", "--format=%H", "--diff-filter=A", "--",
        "tests/evidence/path-cli-2026-11-03/receipt.json",
    ], cwd=ROOT, text=True).strip()
    if not commit:
        return current
    return subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT)


class CliEvidence(unittest.TestCase):
    def test_raw_runs_and_source_hashes(self):
        receipt = json.loads((EVIDENCE / "receipt.json").read_text())
        for path, digest in receipt["inputs"].items():
            self.assertEqual(sha(observed_input(path, digest)), digest, path)
        runs = {run["label"]: run for run in receipt["runs"]}
        self.assertEqual(set(runs), {"default", "test-hooks"})
        for label, count in (("default", 3), ("test-hooks", 5)):
            run = runs[label]
            self.assertEqual(run["exit_code"], 0)
            for stream in ("stdout", "stderr"):
                self.assertEqual(sha((EVIDENCE / f"{label}.{stream}.txt").read_bytes()), run[f"{stream}_sha256"])
            self.assertIn(f"{count} passed; 0 failed", (EVIDENCE / f"{label}.stdout.txt").read_text())
        self.assertNotEqual(runs["default"]["binary_sha256"], runs["test-hooks"]["binary_sha256"])

    def test_declared_registry_inventory_matches_lock(self):
        lock = tomllib.loads((ROOT / "apps/grok-host/Cargo.lock").read_text())
        packages = {(p["name"], p["version"]): p for p in lock["package"] if "source" in p}
        inventory = json.loads((EVIDENCE / "dependencies.json").read_text())["packages"]
        self.assertEqual(set(packages), {(p["name"], p["version"]) for p in inventory})
        for item in inventory:
            locked = packages[item["name"], item["version"]]
            self.assertEqual(item["source"], locked["source"])
            self.assertEqual(item["checksum"], locked["checksum"])
            self.assertTrue(item["declared_license"])


if __name__ == "__main__":
    unittest.main()
