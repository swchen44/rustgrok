"""Check recorded batch correctness inputs, raw streams and local executable hashes."""
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
REL = 'tests/evidence/batch-publication-2026-11-03'
EVIDENCE = ROOT / REL


def sha(data):
    return hashlib.sha256(data).hexdigest()


class BatchPublicationEvidence(unittest.TestCase):
    def test_inputs_streams_and_local_test_executables(self):
        receipt = json.loads((EVIDENCE / 'receipt.json').read_text())
        self.assertEqual(receipt['base_commit'], '39a574cd93d4b631613fa52f1cea9fc32afb5a9a')
        self.assertEqual(receipt['guard']['reserve_bytes'], int(3.5 * 1024**3))
        self.assertTrue(receipt['guard']['not_system_quota'])
        runs = {run['label']: run for run in receipt['runs']}
        self.assertEqual(set(runs), {'default', 'fault'})
        for label, run in runs.items():
            self.assertEqual(run['status'], 'PASS')
            self.assertEqual(run['exit_code'], 0)
            self.assertGreaterEqual(run['lowest_observed_free_bytes'], receipt['guard']['reserve_bytes'])
            for stream in ('stdout', 'stderr'):
                self.assertEqual(sha((EVIDENCE / f'{label}.{stream}.txt').read_bytes()), run[stream + '_sha256'])
            local = Path(run['test_binary_local_path'])
            if local.exists():
                self.assertEqual(local.stat().st_size, run['test_binary_bytes'])
                self.assertEqual(sha(local.read_bytes()), run['test_binary_sha256'])
            self.assertIn(f"{'9' if label == 'default' else '13'} passed; 0 failed", (EVIDENCE / f'{label}.stdout.txt').read_text())
        for path, expected in receipt['inputs'].items():
            data = (ROOT / path).read_bytes()
            if sha(data) != expected:
                commit = subprocess.check_output(['git', 'log', '-1', '--format=%H', '--diff-filter=A', '--', f'{REL}/receipt.json'], cwd=ROOT, text=True).strip()
                self.assertTrue(commit, path)
                data = subprocess.check_output(['git', 'show', f'{commit}:{path}'], cwd=ROOT)
            self.assertEqual(sha(data), expected, path)


if __name__ == '__main__':
    unittest.main()
