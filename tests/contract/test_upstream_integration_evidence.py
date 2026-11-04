"""Check provenance and bytes of the saved independent original token stream."""
import base64
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'tests/evidence/upstream-integration-2026-11-03'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

class UpstreamIntegrationEvidence(unittest.TestCase):
    def test_pinned_sources_fixture_and_complete_original_stream(self):
        provenance = json.loads((EVIDENCE / 'source-provenance.json').read_text())
        receipt = json.loads((EVIDENCE / 'path-tokenizer-receipt.json').read_text())
        tree = json.loads((ROOT / 'upstream/opengrok-source-tree.json').read_text())
        release = json.loads((ROOT / 'upstream/opengrok-release-inventory.json').read_text())
        self.assertEqual(provenance['commit'], tree['commit'])
        self.assertEqual(receipt['upstream_commit'], tree['commit'])
        inventory = {item['path']: item['sha256'] for item in tree['files']}
        self.assertEqual(len(provenance['files']), 7)
        for item in provenance['files']:
            self.assertEqual(item['sha256'], inventory[item['path']])
        fixture = provenance['synthetic_fixture']
        self.assertEqual(sha(ROOT / fixture['path']), fixture['sha256'])
        expected_jars = {Path(item['path']).name: item['sha256'] for item in release['archive']['members']
                         if item['path'].startswith('opengrok-1.14.20/lib/') and item['path'].endswith('.jar')}
        self.assertEqual(receipt['class_path_jars_sha256'], expected_jars)
        self.assertEqual(sha(ROOT / 'tests/evidence/path-analyzer-2026-11-03/PathTokenOracle.java'),
                         receipt['wrapper_source_sha256'])
        self.assertEqual(sha(EVIDENCE / 'path-tokenizer.stdout'), receipt['raw_stdout_sha256'])
        self.assertEqual(sha(EVIDENCE / 'path-tokenizer.stderr'), receipt['raw_stderr_sha256'])
        self.assertEqual(receipt['exit_code'], 0)
        self.assertEqual(base64.b64decode(receipt['input_base64']).decode(), receipt['input_utf8'])
        self.assertEqual(hashlib.sha256(receipt['input_utf8'].encode()).hexdigest(), receipt['input_sha256'])
        self.assertEqual(receipt['input_utf8'], 'alpha/beta/gamma/delta.ext')
        tokens = [(int(fields[0]), int(fields[1]), int(fields[2]), base64.b64decode(fields[3]).decode())
                  for line in (EVIDENCE / 'path-tokenizer.stdout').read_text().splitlines()
                  if not line.startswith('END\t') for fields in [line.split('\t')]]
        self.assertEqual(tokens, [(0, 0, 5, 'alpha'), (1, 6, 10, 'beta'), (2, 11, 16, 'gamma'),
                                  (3, 17, 22, 'delta'), (4, 23, 24, '.'), (5, 24, 27, 'ext')])

if __name__ == '__main__':
    unittest.main()
