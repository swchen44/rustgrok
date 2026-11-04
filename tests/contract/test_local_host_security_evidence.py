"""Validate host security-run evidence and Cargo lock inventory."""
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "tests/evidence/local-path-host-security-2026-11-03"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def input_bytes(path, expected):
    current = (ROOT / path).read_bytes()
    if sha(current) == expected:
        return current
    commit = subprocess.check_output([
        "git", "log", "-1", "--format=%H", "--diff-filter=A", "--",
        "tests/evidence/local-path-host-security-2026-11-03/receipt.json",
    ], cwd=ROOT, text=True).strip()
    if not commit:
        return current
    return subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT)


class HostSecurityEvidence(unittest.TestCase):
    def test_receipt_and_streams(self):
        receipt = json.loads((EVIDENCE / "receipt.json").read_text())
        self.assertEqual(receipt["sdk_acl_type_extended"], 256)
        self.assertEqual(receipt["private_tmp_observed_fpathconf"], {"extended_security": 1, "auth_opaque": 0})
        self.assertEqual(len(receipt["sdk_acl_header_sha256"]), 64)
        self.assertEqual(len(receipt["sdk_unistd_header_sha256"]), 64)
        for path, expected in receipt["inputs"].items():
            self.assertEqual(sha(input_bytes(path, expected)), expected, path)
        runs = {r["label"]: r for r in receipt["runs"]}
        self.assertEqual(set(runs), {"default", "test-hooks"})
        for label, count in (("default", 12), ("test-hooks", 20)):
            run = runs[label]
            self.assertEqual(run["exit_code"], 0)
            retained = run["binary_retention"]
            self.assertEqual(retained["sha256"], run["binary_sha256"])
            self.assertIn(run["binary_sha256"], retained["local_path"])
            local = Path(retained["local_path"]).expanduser()
            if local.exists():
                self.assertEqual(local.stat().st_size, retained["bytes"])
                self.assertEqual(sha(local.read_bytes()), retained["sha256"])
            for stream in ("stdout", "stderr"):
                self.assertEqual(sha((EVIDENCE / f"{label}.{stream}.txt").read_bytes()), run[f"{stream}_sha256"])
            self.assertIn(f"{count} passed; 0 failed", (EVIDENCE / f"{label}.stdout.txt").read_text())
        self.assertNotEqual(runs["default"]["binary_sha256"], runs["test-hooks"]["binary_sha256"])

    def test_registry_lock_inventory(self):
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
