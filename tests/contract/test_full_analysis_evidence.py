# Copyright (c) 2026 swchen44
# SPDX-License-Identifier: MIT
"""Verify committed FULL oracle capture bytes and their declared inputs."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'tests/evidence/full-analysis-2026-11-04'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class FullEvidence(unittest.TestCase):
    def test_original_runs_and_inputs(self):
        for directory, expected_count in ((EVIDENCE, 31), (EVIDENCE / 'extra', 4)):
            receipt = json.loads((directory / 'receipt.json').read_text())
            self.assertEqual(receipt['upstream_commit'], '3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3')
            self.assertEqual(len(receipt['cases']), expected_count)
            self.assertEqual(receipt['helper_sha256'], sha(EVIDENCE / 'FullAnalysisOracle.java'))
            self.assertEqual(receipt['javac_exit'], 0)
            inventory = json.loads((ROOT / 'upstream/opengrok-source-tree.json').read_text())
            pinned = {item['path']: item['sha256'] for item in inventory['files']}
            for path, digest in receipt['upstream_source_sha256'].items():
                self.assertEqual(pinned[path], digest, path)
            for case in receipt['cases']:
                with self.subTest(case=case['name']):
                    self.assertEqual(case['exit'], 0)
                    self.assertEqual(case['input_sha256'], sha(ROOT / case['input']))
                    for stream in ('stdout', 'stderr'):
                        self.assertEqual(case[stream + '_sha256'], sha(directory / case[stream]))

    def test_sigma_and_bytecode_streams(self):
        sigma = json.loads((EVIDENCE / 'sigma-matrix-receipt.json').read_text())
        self.assertEqual(sigma['exit'], 0)
        self.assertEqual(sigma['input_sha256'], sha(ROOT / 'tests/fixtures/full-analysis-2026-11-04/sigma-matrix.txt'))
        for stream in ('stdout', 'stderr'):
            self.assertEqual(sigma[stream + '_sha256'], sha(EVIDENCE / ('sigma-matrix-query.' + stream)))
        bytecode = json.loads((EVIDENCE / 'lucene-bytecode-receipt.json').read_text())
        for run in bytecode['runs']:
            self.assertEqual(run['exit'], 0)
            for stream in ('stdout', 'stderr'):
                self.assertEqual(run[stream + '_sha256'], sha(EVIDENCE / (run['label'] + '.' + stream)))


if __name__ == '__main__':
    unittest.main()
