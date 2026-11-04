# SPDX-License-Identifier: MIT
# Copyright (c) 2026 swchen44
"""Integrity checks for the raw original reader/FULL/query/postings captures."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'tests/evidence/full-postings-2026-11-05'


def sha(data):
    return hashlib.sha256(data).hexdigest()


class FullPostingsEvidence(unittest.TestCase):
    def test_original_inputs_outputs_and_sources(self):
        receipt = json.loads((EVIDENCE / 'oracle-receipt.json').read_text())
        self.assertEqual(receipt['upstream_commit'], '3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3')
        self.assertEqual(receipt['helper_sha256'], sha((EVIDENCE / 'FullPostingsOracle.java').read_bytes()))
        self.assertEqual(receipt['javac_exit'], 0)
        inventory = json.loads((ROOT / 'upstream/opengrok-source-tree.json').read_text())
        pinned = {item['path']: item['sha256'] for item in inventory['files']}
        for path, digest in receipt['upstream_source_sha256'].items():
            self.assertEqual(digest, pinned[path])
        self.assertEqual(len(receipt['cases']), 18)
        for case in receipt['cases']:
            with self.subTest(case=case['name']):
                self.assertEqual(case['exit'], 0)
                if 'input_hex_fixture' in case:
                    data = bytes.fromhex((ROOT / case['input_hex_fixture']).read_text())
                else:
                    data = (ROOT / case['repo_input']).read_bytes()
                self.assertEqual(sha(data), case['input_sha256'])
                for stream in ('stdout', 'stderr'):
                    self.assertEqual(sha((EVIDENCE / case[stream]).read_bytes()), case[stream + '_sha256'])
                lines = (EVIDENCE / case['stdout']).read_text().splitlines()
                self.assertTrue(lines[0].startswith('READER\t'))
                self.assertTrue(any(line.startswith('QUERY\t') for line in lines))

    def test_partial_status_and_reserve_failure_are_preserved(self):
        status = json.loads((EVIDENCE / 'checkpoint-verification.json').read_text())
        self.assertEqual(status['status'], 'partial_checkpoint_verification_blocked_by_disk_reserve')
        self.assertEqual(status['product_tests_80'], 'planned_not_run')
        blocked = json.loads((EVIDENCE / 'index-default-suite.json').read_text())
        self.assertEqual(blocked['status'], 'BLOCKED_RESERVE')
        self.assertLess(blocked['minimum_sampled_free_bytes'], blocked['reserve_bytes'])


if __name__ == '__main__':
    unittest.main()
