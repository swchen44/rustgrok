"""Validate scoped macOS batch scanner and CLI test receipts."""
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
REL = 'tests/evidence/batch-source-cli-2026-11-03'
EVIDENCE = ROOT / REL


def sha(data):
    return hashlib.sha256(data).hexdigest()


class BatchSourceEvidence(unittest.TestCase):
    def test_raw_streams_inputs_and_locally_retained_binaries(self):
        receipt = json.loads((EVIDENCE / 'receipt.json').read_text())
        self.assertEqual(receipt['base_commit'], 'a58c4a5333df8791bb88d01e8cbe31fc60e3d953')
        self.assertEqual(receipt['guard']['reserve_bytes'], int(2.5 * 1024**3))
        self.assertTrue(receipt['guard']['not_system_quota'])
        runs = {r['label']: r for r in receipt['runs']}
        self.assertEqual(set(runs), {'index-default', 'index-fault', 'cli-default', 'cli-hooks'})
        for label, run in runs.items():
            self.assertEqual(run['status'], 'PASS')
            self.assertEqual(run['exit_code'], 0)
            self.assertGreaterEqual(run['lowest_observed_free_bytes'], receipt['guard']['reserve_bytes'])
            for stream in ('stdout', 'stderr'):
                self.assertEqual(sha((EVIDENCE / f'{label}.{stream}.txt').read_bytes()), run[stream + '_sha256'])
            for kind in ('test_binary', 'product_binary'):
                if kind in run:
                    item = run[kind]
                    local = Path(item['local_path'])
                    if local.exists():
                        self.assertEqual(local.stat().st_size, item['bytes'])
                        self.assertEqual(sha(local.read_bytes()), item['sha256'])
            output = (EVIDENCE / f'{label}.stdout.txt').read_text()
            count = {'index-default': 11, 'index-fault': 15, 'cli-default': 4, 'cli-hooks': 5}[label]
            self.assertIn(f'{count} passed; 0 failed', output)
        for path, expected in receipt['inputs'].items():
            data = (ROOT / path).read_bytes()
            if sha(data) != expected:
                commit = subprocess.check_output(['git','log','-1','--format=%H','--diff-filter=A','--',f'{REL}/receipt.json'],cwd=ROOT,text=True).strip()
                self.assertTrue(commit, path)
                data = subprocess.check_output(['git','show',f'{commit}:{path}'],cwd=ROOT)
            self.assertEqual(sha(data),expected,path)


    def test_pre_lint_capture_retains_original_streams_and_source_identity(self):
        final = json.loads((EVIDENCE / 'receipt.json').read_text())
        prior = json.loads((EVIDENCE / 'pre-lint/receipt.json').read_text())
        source = 'crates/path-index/src/durable/scanner.rs'
        self.assertEqual(set(final['inputs']), set(prior['inputs']))
        for path in final['inputs']:
            if path != source:
                self.assertEqual(final['inputs'][path], prior['inputs'][path], path)
        data = (ROOT / source).read_bytes()
        if sha(data) != final['inputs'][source]:
            commit = subprocess.check_output(['git', 'log', '-1', '--format=%H', '--diff-filter=A', '--', f'{REL}/receipt.json'], cwd=ROOT, text=True).strip()
            self.assertTrue(commit)
            data = subprocess.check_output(['git', 'show', f'{commit}:{source}'], cwd=ROOT)
        marker = b'#[allow(clippy::too_many_arguments)] // traversal state and cancellation are separate inputs.\n'
        self.assertEqual(data.count(marker), 1)
        self.assertEqual(sha(data.replace(marker, b'', 1)), prior['inputs'][source])
        self.assertEqual({r['label'] for r in prior['runs']}, {'index-default', 'index-fault', 'cli-default', 'cli-hooks'})
        for run in prior['runs']:
            self.assertEqual(run['status'], 'PASS')
            self.assertEqual(run['exit_code'], 0)
            for stream in ('stdout', 'stderr'):
                path = EVIDENCE / 'pre-lint' / f"{run['label']}.{stream}.txt"
                self.assertEqual(sha(path.read_bytes()), run[stream + '_sha256'])
            for kind in ('test_binary', 'product_binary'):
                if kind in run:
                    item = run[kind]
                    local = Path(item['local_path'])
                    if local.exists():
                        self.assertEqual(sha(local.read_bytes()), item['sha256'])


if __name__ == '__main__':
    unittest.main()
