"""Validate scoped batch-host tests and the guard-interrupted rerun."""
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
REL = 'tests/evidence/batch-host-2026-11-04'
EVIDENCE = ROOT / REL


def sha(data):
    return hashlib.sha256(data).hexdigest()


def verify_binary(test, item):
    local = Path(item['local_path'])
    if local.exists():
        test.assertEqual(local.stat().st_size, item['bytes'])
        test.assertEqual(sha(local.read_bytes()), item['sha256'])


class BatchHostEvidence(unittest.TestCase):
    def test_runs_inputs_guard_and_retained_binaries(self):
        raw = (EVIDENCE / 'receipt.json').read_bytes()
        receipt = json.loads(raw)
        self.assertEqual(receipt['base_commit'], 'daf04dcf2dc1bbc0032ba43ecb82d9ba9df3ca80')
        self.assertEqual(receipt['guard']['reserve_bytes'], int(2.5 * 1024**3))
        self.assertTrue(receipt['guard']['not_system_quota'])
        runs = {item['label']: item for item in receipt['runs']}
        self.assertEqual(set(runs), {'host-default','host-hooks','index-default','index-fault'})
        for label, run in runs.items():
            for stream in ('stdout','stderr'):
                self.assertEqual(sha((EVIDENCE / f'{label}.{stream}.txt').read_bytes()), run[stream+'_sha256'])
            if label == 'index-fault':
                self.assertEqual(run['status'], 'BLOCKED_RESERVE')
                self.assertLess(run['lowest_observed_free_bytes'], receipt['guard']['reserve_bytes'])
            else:
                self.assertEqual(run['status'], 'PASS')
                self.assertEqual(run['exit_code'], 0)
                self.assertGreaterEqual(run['lowest_observed_free_bytes'], receipt['guard']['reserve_bytes'])
                verify_binary(self, run['test_binary'])
                if 'product_binary' in run:
                    verify_binary(self, run['product_binary'])
        for path, expected in receipt['inputs'].items():
            data = (ROOT / path).read_bytes()
            if sha(data) != expected:
                commit = subprocess.check_output(['git','log','-1','--format=%H','--diff-filter=A','--',f'{REL}/receipt.json'],cwd=ROOT,text=True).strip()
                self.assertTrue(commit,path)
                data = subprocess.check_output(['git','show',f'{commit}:{path}'],cwd=ROOT)
            self.assertEqual(sha(data), expected, path)
        cleanup = (EVIDENCE / 'incremental-cleanup.json').read_bytes()
        clean = json.loads(cleanup)
        self.assertEqual(clean['status'], 'REMOVED')
        self.assertEqual(clean['target'], '/Users/swchen.tw/git/rustgrok-durable-path/crates/path-index/target/debug/incremental')
        self.assertTrue(clean['not_symlink'])
        self.assertEqual(clean['lsof_matching_open_files'], 0)
        rerun = json.loads((EVIDENCE / 'index-fault-rerun.json').read_text())
        self.assertEqual(rerun['prior_receipt_sha256'], sha(raw))
        self.assertEqual(rerun['cleanup_receipt_sha256'], sha(cleanup))
        self.assertEqual(rerun['command'], runs['index-fault']['command'])
        self.assertEqual(rerun['status'],'PASS')
        self.assertEqual(rerun['exit_code'],0)
        self.assertGreaterEqual(rerun['lowest_observed_free_bytes'], receipt['guard']['reserve_bytes'])
        for stream in ('stdout','stderr'):
            self.assertEqual(sha((EVIDENCE / f'index-fault-rerun.{stream}.txt').read_bytes()), rerun[stream+'_sha256'])
        verify_binary(self, rerun['test_binary'])
        for label,count in [('host-default',3),('host-hooks',5),('index-default',11),('index-fault-rerun',17)]:
            self.assertIn(f'{count} passed; 0 failed', (EVIDENCE / f'{label}.stdout.txt').read_text())


if __name__ == '__main__':
    unittest.main()
