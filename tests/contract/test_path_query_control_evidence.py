"""Verify the copied independent review and the scoped native fix tests."""
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
REL = 'tests/evidence/path-query-control-review-2026-11-03'
EVIDENCE = ROOT / REL


def sha(data):
    return hashlib.sha256(data).hexdigest()


class ControlEvidence(unittest.TestCase):
    def test_independent_reviewer_streams(self):
        subset = json.loads((EVIDENCE / 'reviewer-subset.json').read_text())
        self.assertEqual(subset['source_head'], '89f9ae6a21d51d848977852ab5c6705dd3724900')
        for path, expected in subset['file_sha256'].items():
            self.assertEqual(sha((EVIDENCE / path).read_bytes()), expected, path)
        local = Path(subset['original_receipt_local_path'])
        if local.exists():
            self.assertEqual(sha(local.read_bytes()), subset['original_receipt_sha256'])
        original = json.loads((EVIDENCE / 'original-http/receipt.json').read_text())
        comparison = json.loads((EVIDENCE / 'comparison.json').read_text())
        for i, op in enumerate(('AND', 'OR', 'NOT')):
            self.assertEqual(original['cases'][i]['query'], f'alpha {op}\fbeta')
            self.assertEqual(original['cases'][i]['status'], 200)
            result = json.loads((EVIDENCE / 'original-http' / original['cases'][i]['body']).read_text())
            self.assertEqual(result['resultCount'], 0)
            self.assertEqual(result['results'], {})
            self.assertFalse(comparison[i]['equivalent'])
            self.assertEqual(len(comparison[i]['rust_paths']), (1, 4, 2)[i])
        self.assertTrue(comparison[3]['equivalent'])
        self.assertTrue(comparison[4]['equivalent'])

    def test_test_receipt(self):
        receipt = json.loads((EVIDENCE / 'rust-test-receipt.json').read_text())
        for run in receipt['runs']:
            self.assertEqual(run['exit_code'], 0)
            for stream in ('stdout', 'stderr'):
                self.assertEqual(sha((EVIDENCE / f"{run['label']}.{stream}.txt").read_bytes()), run[stream + '_sha256'])
            if 'binary_local_path' in run:
                binary = Path(run['binary_local_path'])
                if binary.exists():
                    self.assertEqual(sha(binary.read_bytes()), run['binary_sha256'])
        self.assertEqual({r['label'] for r in receipt['runs']}, {'path-index', 'host-default', 'host-hooks'})
        for path, expected in receipt['inputs'].items():
            actual = (ROOT / path).read_bytes()
            if sha(actual) != expected:
                commit = subprocess.check_output(['git', 'log', '-1', '--format=%H', '--diff-filter=A', '--', f'{REL}/rust-test-receipt.json'], cwd=ROOT, text=True).strip()
                self.assertTrue(commit, path)
                actual = subprocess.check_output(['git', 'show', f'{commit}:{path}'], cwd=ROOT)
            self.assertEqual(sha(actual), expected, path)


if __name__ == '__main__':
    unittest.main()
