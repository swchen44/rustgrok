# SPDX-License-Identifier: MIT
# Copyright (c) 2026 swchen44
"""Integrity of the native FULL host correctness receipts; no runtime acceptance inference."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'tests/evidence/full-host-2026-11-05'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class FullHostEvidence(unittest.TestCase):
    def test_raw_runs_source_and_fixture_hashes(self):
        receipt = json.loads((EVIDENCE / 'receipt.json').read_text())
        self.assertEqual(receipt['base_commit'], '603945346fd8523ca31ad9839b6ea477b866fff0')
        self.assertEqual(receipt['upstream_commit'], '3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3')
        self.assertEqual(receipt['new_java_oracle_executions'], 0)
        for section in ('source_sha256', 'public_fixture_sha256'):
            for path, digest in receipt[section].items():
                self.assertEqual(sha(ROOT / path), digest, path)
        self.assertEqual(sha(EVIDENCE / 'executed-runner.py'), receipt['runner_sha256'])
        for name, run in receipt['runs'].items():
            with self.subTest(run=name):
                self.assertEqual(sha(EVIDENCE / run['receipt']), run['receipt_sha256'])
                for stream in ('stdout', 'stderr'):
                    self.assertEqual(sha(EVIDENCE / run[stream]), run[stream + '_sha256'])

    def test_completed_scoped_checks_and_distinct_red_failures(self):
        receipt = json.loads((EVIDENCE / 'receipt.json').read_text())
        self.assertEqual(receipt['product_tests_80'], 'planned_not_run')
        for name in ('host-publication-green', 'publication-green', 'default-host-regression',
                     'hooks-host-regression', 'full-store-fault-regression', 'index-clippy', 'cli-clippy'):
            run = json.loads((EVIDENCE / receipt['runs'][name]['receipt']).read_text())
            self.assertEqual(run['status'], 'passed', name)
            self.assertEqual(run['exit'], 0)
            self.assertEqual(run['cargo_incremental'], '0')
            self.assertEqual(run['cargo_build_jobs'], '1')
            self.assertGreaterEqual(run['minimum_sampled_free_bytes'], run['reserve_bytes'])
        for name in ('tracer-red', 'publication-red', 'host-publication-red'):
            self.assertEqual(receipt['runs'][name]['status'], 'failed')
            self.assertIn(name, receipt['failure_explanations'])


if __name__ == '__main__':
    unittest.main()
