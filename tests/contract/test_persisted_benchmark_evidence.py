"""Recheck preserved sample bodies using the stricter gate, without rerunning timing."""
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from search_benchmark_gate import comparison, load_response


class PersistedEvidence(unittest.TestCase):
    def test_tokenized_path_samples_match_strict_preflight(self):
        directory = ROOT / 'tests/evidence/live-path-benchmark-tokenized-2026-11-01/run'
        report = json.loads((directory / 'report.json').read_text())
        reference = load_response(directory / report['preflight'][0]['body'])[1]
        self.assertEqual(len(report['samples']), 200)
        for sample in report['samples']:
            with self.subTest(body=sample['body']):
                self.assertEqual(sample['status'], 200)
                self.assertIsNone(sample['error'])
                self.assertTrue(comparison(reference, load_response(directory / sample['body'])[1]))


if __name__ == '__main__': unittest.main()
