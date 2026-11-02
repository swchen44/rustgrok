"""Validate recorded native host test streams and historical input hashes."""
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "tests/evidence/local-path-host-2026-11-03"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def input_bytes(path, expected):
    current = (ROOT / path).read_bytes()
    if sha(current) == expected:
        return current
    commit = subprocess.check_output([
        "git", "log", "-1", "--format=%H", "--diff-filter=A", "--",
        "tests/evidence/local-path-host-2026-11-03/receipt.json",
    ], cwd=ROOT, text=True).strip()
    if not commit:
        return current
    return subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT)


class HostEvidence(unittest.TestCase):
    def test_input_and_stream_hashes(self):
        receipt = json.loads((EVIDENCE / "receipt.json").read_text())
        for path, digest in receipt["inputs"].items():
            self.assertEqual(sha(input_bytes(path, digest)), digest, path)
        runs = {run["label"]: run for run in receipt["runs"]}
        self.assertEqual(set(runs), {"default", "test-hooks"})
        for label, count in (("default", 9), ("test-hooks", 15)):
            run = runs[label]
            self.assertEqual(run["exit_code"], 0)
            for stream in ("stdout", "stderr"):
                self.assertEqual(sha((EVIDENCE / f"{label}.{stream}.txt").read_bytes()), run[f"{stream}_sha256"])
            self.assertIn(f"{count} passed; 0 failed", (EVIDENCE / f"{label}.stdout.txt").read_text())
        self.assertNotEqual(runs["default"]["binary_sha256"], runs["test-hooks"]["binary_sha256"])


if __name__ == "__main__":
    unittest.main()
