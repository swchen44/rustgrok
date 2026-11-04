"""Check retained startup-diagnosis streams and their scoped receipts."""
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
REL = "tests/evidence/local-host-startup-diagnosis-2026-11-03"
EVIDENCE = ROOT / REL


def sha(data):
    return hashlib.sha256(data).hexdigest()


def recorded_input(path, expected):
    current = (ROOT / path).read_bytes()
    if sha(current) == expected:
        return current
    commit = subprocess.check_output(
        ["git", "log", "-1", "--format=%H", "--diff-filter=A", "--", f"{REL}/receipt.json"],
        cwd=ROOT, text=True,
    ).strip()
    if not commit:
        return current
    return subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT)


class StartupDiagnosisEvidence(unittest.TestCase):
    def test_recorded_inputs_and_raw_streams(self):
        receipt = json.loads((EVIDENCE / "receipt.json").read_text())
        for path, expected in receipt["inputs"].items():
            self.assertEqual(sha(recorded_input(path, expected)), expected, path)
        for path, expected in receipt["raw_file_sha256"].items():
            self.assertEqual(sha((ROOT / path).read_bytes()), expected, path)
        self.assertEqual(sha((EVIDENCE / "reviewer-receipt-subset.json").read_bytes()),
                         receipt["reviewer_subset_sha256"])
        for run in receipt["runs"]:
            self.assertEqual(run["exit_code"], 0)
            for stream in ("stdout", "stderr"):
                data = (EVIDENCE / f'{run["label"]}.{stream}.txt').read_bytes()
                self.assertEqual(sha(data), run[f"{stream}_sha256"])
            local = Path(run["binary_local_path"]).expanduser()
            if local.exists():
                self.assertEqual(sha(local.read_bytes()), run["binary_sha256"])
        for label, cli, host in (("post-default", 3, 14), ("post-hooks", 5, 22)):
            output = (EVIDENCE / f"{label}.stdout.txt").read_text()
            self.assertIn(f"{cli} passed; 0 failed", output)
            self.assertIn(f"{host} passed; 0 failed", output)
        self.assertEqual(receipt["post_exact"]["exit_code"], 0)
        for stream in ("stdout", "stderr"):
            self.assertEqual(sha((EVIDENCE / f"post-exact.{stream}.txt").read_bytes()),
                             receipt["post_exact"][f"{stream}_sha256"])

    def test_reviewer_failure_and_successful_stage_observation(self):
        subset = json.loads((EVIDENCE / "reviewer-receipt-subset.json").read_text())
        for run in subset["runs"]:
            self.assertEqual(run["exit_code"], 101)
            for stream in ("stdout", "stderr"):
                data = (EVIDENCE / f'reviewer-{run["name"]}.{stream}.txt').read_bytes()
                self.assertEqual(sha(data), run[f"{stream}_sha256"])
            self.assertIn("Err(Timeout)", (EVIDENCE / f'reviewer-{run["name"]}.stdout.txt').read_text())
        full = Path(subset["original_receipt_local_path"]).expanduser()
        if full.exists():
            self.assertEqual(sha(full.read_bytes()), subset["original_receipt_sha256"])
        observed = (EVIDENCE / "stage-observation.stderr.txt").read_text()
        stages = ("control-start", "control-done", "store-open-start", "decode-start", "decode-done",
                  "recovery-sync-start", "recovery-sync-done", "store-open-done", "bind-start",
                  "bind-done", "readiness-flushed")
        positions = [observed.index(f"[DEBUG-stage] {stage}\n") for stage in stages]
        self.assertEqual(positions, sorted(positions))


if __name__ == "__main__":
    unittest.main()
