"""Verify retained original Unicode and bounded sigma correctness evidence."""
import base64
import hashlib
import itertools
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'tests/evidence/unicode-path-2026-11-03'

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

class UnicodeEvidence(unittest.TestCase):
    def test_original_streams_and_sigma_cartesian_coverage(self):
        original = json.loads((EVIDENCE / 'oracle.json').read_text(encoding='utf-8'))
        sigma = json.loads((EVIDENCE / 'sigma-contexts-receipt.json').read_text(encoding='utf-8'))
        tree = json.loads((ROOT / 'upstream/opengrok-source-tree.json').read_text(encoding='utf-8'))
        release = json.loads((ROOT / 'upstream/opengrok-release-inventory.json').read_text(encoding='utf-8'))
        self.assertEqual(original['upstream_commit'], tree['commit'])
        self.assertEqual(sigma['upstream_commit'], tree['commit'])
        jars = {Path(x['path']).name: x['sha256'] for x in release['archive']['members']
                if x['path'].startswith('opengrok-1.14.20/lib/') and x['path'].endswith('.jar')}
        self.assertEqual(original['classpath_jar_sha256'], jars)
        self.assertEqual(sigma['classpath_jar_sha256'], jars)
        self.assertEqual(original['java_sha256'], sigma['java_sha256'])
        self.assertEqual(sha((ROOT / 'tests/evidence/path-analyzer-2026-11-03/PathTokenOracle.java').read_bytes()),
                         original['wrapper_source_sha256'])
        self.assertEqual(sha((EVIDENCE / 'SigmaContextOracle.java').read_bytes()), sigma['wrapper_source_sha256'])
        self.assertEqual(len(original['cases']), 24)
        for case in original['cases']:
            self.assertEqual(sha(case['input'].encode()), case['input_sha256'])
            self.assertEqual(sha(case['raw_stdout'].encode()), case['stdout_sha256'])
            self.assertEqual((case['exit_code'], case['raw_stderr']), (0, ''))
        cjk = original['cases'][-1]
        self.assertEqual(cjk['input'], ''.join(chr(c) for c in range(0x4e00, 0xa000)))
        stream = cjk['raw_stdout'].splitlines()
        fields = stream[0].split('\t')
        self.assertEqual(fields[:3], ['0', '0', '20992'])
        self.assertEqual(base64.b64decode(fields[3]).decode(), cjk['input'])
        self.assertEqual(stream[1], 'END\t20994\t20994')
        raw = (EVIDENCE / 'sigma-contexts.stdout').read_bytes()
        self.assertEqual(sha(raw), sigma['raw_stdout_sha256'])
        self.assertEqual(sha((EVIDENCE / 'sigma-contexts.stderr').read_bytes()), sigma['raw_stderr_sha256'])
        rows = [line.split('\t') for line in raw.decode().splitlines()]
        self.assertEqual(len(rows), sigma['cases'])
        self.assertEqual(len(rows), 3600)
        observed = [base64.b64decode(row[0]).decode() for row in rows]
        sides = [''] + list('ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzİΑαΟοσς')
        expected = {left + 'Σ' + right for left, right in itertools.product(sides, repeat=2)}
        self.assertEqual(len(set(observed)), len(observed))
        self.assertEqual(set(observed), expected)
        self.assertTrue(all(row[1:3] == ['0', '0'] for row in rows))

if __name__ == '__main__':
    unittest.main()
