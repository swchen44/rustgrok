"""Check the limited path-index correctness receipt and pinned provenance."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'tests/evidence/path-index-2026-11-03'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

class PathIndexEvidence(unittest.TestCase):
    def test_sources_raw_reference_and_test_run_hashes(self):
        provenance = json.loads((EVIDENCE / 'provenance.json').read_text())
        receipt = json.loads((EVIDENCE / 'test-receipt.json').read_text())
        tree = json.loads((ROOT / 'upstream/opengrok-source-tree.json').read_text())
        self.assertEqual(provenance['upstream_commit'], tree['commit'])
        inventory = {item['path']: item['sha256'] for item in tree['files']}
        self.assertEqual(len(provenance['source_files']), 2)
        for item in provenance['source_files']:
            self.assertEqual(item['sha256'], inventory[item['path']])
        self.assertEqual(len(provenance['raw_reference_files']), 3)
        for item in provenance['raw_reference_files']:
            self.assertEqual(item['sha256'], sha(ROOT / item['path']))
        self.assertEqual(receipt['exit_code'], 0)
        self.assertEqual(receipt['stdout_sha256'], sha(EVIDENCE / 'test.stdout'))
        self.assertEqual(receipt['stderr_sha256'], sha(EVIDENCE / 'test.stderr'))
        self.assertIn('14 passed; 0 failed', (EVIDENCE / 'test.stdout').read_text())
        for name, digest in receipt['inputs_sha256'].items():
            self.assertEqual(digest, sha(ROOT / name))

if __name__ == '__main__':
    unittest.main()
