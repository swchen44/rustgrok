"""Recheck the retained partial experiment without running servers or builds."""
import hashlib
import json
from pathlib import Path
import sys
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from search_benchmark_gate import strict_equal
from sequential_preflight import equivalent

EVIDENCE = ROOT / 'tests/evidence/synthetic-20m-cache-profile-2026-11-02'


class CacheProfileEvidence(unittest.TestCase):
    def test_archive_hashes_preflight_and_timed_bodies(self):
        archive = EVIDENCE / 'raw-observations.zip'
        manifest = json.loads((EVIDENCE / 'raw-observations-manifest.json').read_text())
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), manifest['archive_sha256'])
        with zipfile.ZipFile(archive) as z:
            self.assertEqual(set(z.namelist()), {entry['path'] for entry in manifest['entries']})
            self.assertEqual(len(z.namelist()), len(manifest['entries']))
            for entry in manifest['entries']:
                raw = z.read(entry['path'])
                self.assertEqual((len(raw), hashlib.sha256(raw).hexdigest()),
                                 (entry['bytes'], entry['sha256']))
            baseline = json.loads(z.read('rustgrok-opt-baseline-preflight/sequential-preflight.json'))
            cached = json.loads(z.read('rustgrok-opt-cache-preflight/sequential-preflight.json'))
            for case in ('single-hit', 'no-hit'):
                for proof in (baseline, cached):
                    self.assertEqual(proof[case]['status'], 'MATCHED_PREFLIGHT_NOT_TIMED')
                    self.assertTrue(equivalent(proof[case]['oracle'], proof[case]['candidate']))
                expected = dict(baseline[case]['oracle']['followup']['body'])
                expected.pop('time', None)
                for root, role in (('rustgrok-opt-oracle-baseline-measure', 'oracle'),
                                   ('rustgrok-opt-baseline-measure', 'candidate')):
                    prefix = f'{root}/measure-{role}-{case}/'
                    report = json.loads(z.read(prefix + 'report.json'))
                    self.assertEqual(report['status'], 'COMPLETED_RESTRICTED_ENDPOINT')
                    self.assertEqual(len(report['samples']), 100)
                    for sample in report['samples']:
                        self.assertEqual(sample['status'], 200)
                        self.assertIsNone(sample['error'])
                        body = json.loads(z.read(prefix + sample['body']))
                        body.pop('time', None)
                        self.assertTrue(strict_equal(body, expected))
                        self.assertEqual(list(body['results']), list(expected['results']))
            for preflight in ('baseline', 'cache'):
                report = json.loads(z.read(f'rustgrok-opt-{preflight}-preflight/report.json'))
                self.assertEqual(len(report['cases']), 8)
                self.assertTrue(all(not case['body_matches_except_time'] for case in report['cases']))
            stopped = json.loads(z.read('rustgrok-profile-test/oracle-cache-measure-receipt.json'))
            self.assertEqual(stopped['status'], 'STOPPED_DISK_RESERVE')
            self.assertTrue(stopped['group_stopped'])

if __name__ == '__main__':
    unittest.main()
