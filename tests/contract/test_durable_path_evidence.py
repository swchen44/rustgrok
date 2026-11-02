"""Validate persisted component test evidence and its registry lock provenance."""
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "tests/evidence/durable-path-2026-11-03"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class DurableEvidence(unittest.TestCase):
    def test_receipt_and_raw_output(self):
        receipt = json.loads((EVIDENCE / "receipt.json").read_text())
        self.assertEqual(receipt["exit_code"], 0)
        for name, digest in receipt["files"].items():
            self.assertEqual(sha(ROOT / name), digest, name)
        for stream in ("stdout", "stderr"):
            self.assertEqual(sha(EVIDENCE / f"durable.{stream}.txt"), receipt[f"{stream}_sha256"])
        self.assertIn("26 passed; 0 failed", (EVIDENCE / "durable.stdout.txt").read_text())
        self.assertIn("fault-injection", receipt["command"])

    def test_dependency_inventory_matches_lock(self):
        lock = tomllib.loads((ROOT / "crates/path-index/Cargo.lock").read_text())
        registry = {(p["name"], p["version"]): p for p in lock["package"] if "source" in p}
        inventory = json.loads((EVIDENCE / "dependencies.json").read_text())["packages"]
        self.assertEqual(set(registry), {(p["name"], p["version"]) for p in inventory})
        for p in inventory:
            locked = registry[p["name"], p["version"]]
            self.assertEqual(p["checksum"], locked["checksum"])
            self.assertEqual(p["source"], locked["source"])
            self.assertTrue(p["declared_license"])


if __name__ == "__main__":
    unittest.main()
