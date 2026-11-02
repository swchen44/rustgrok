"""Compare the public Rust entry point with retained original raw token streams."""
import base64
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[3]
CRATE = ROOT / 'crates/analysis'
FIXTURE = ROOT / 'tests/evidence/path-analyzer-2026-11-03/oracle.json'

class PathOracle(unittest.TestCase):
    def test_oracle_wrapper_and_release_classpath_match_recorded_sources(self):
        oracle = json.loads(FIXTURE.read_text(encoding='utf-8'))
        source = json.loads((ROOT / 'upstream/opengrok-source-tree.json').read_text(encoding='utf-8'))
        release = json.loads((ROOT / 'upstream/opengrok-release-inventory.json').read_text(encoding='utf-8'))
        self.assertEqual(oracle['upstream_commit'], source['commit'])
        upstream_path = 'opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/PathTokenizer.java'
        self.assertEqual(next(x['sha256'] for x in source['files'] if x['path'] == upstream_path),
                         oracle['path_tokenizer_source_sha256'])
        self.assertEqual(hashlib.sha256((FIXTURE.parent / 'PathTokenOracle.java').read_bytes()).hexdigest(),
                         oracle['oracle_wrapper_sha256'])
        jars = {Path(x['path']).name: x['sha256'] for x in release['archive']['members']
                if x['path'].startswith('opengrok-1.14.20/lib/') and x['path'].endswith('.jar')}
        self.assertEqual(jars, oracle['classpath_jar_sha256'])
        self.assertTrue(all(case['exit_code'] == 0 and case['raw_stderr'] == '' for case in oracle['cases']))

    def test_original_raw_streams_match_supported_cases(self):
        subprocess.run(['cargo', 'build', '--offline', '--locked', '--quiet', '--manifest-path',
                        str(CRATE / 'Cargo.toml'), '--example', 'path_tokens'], check=True)
        binary = CRATE / 'target/debug/examples/path_tokens'
        oracle = json.loads(FIXTURE.read_text(encoding='utf-8'))
        self.assertEqual(len(oracle['cases']), 14)
        for case in oracle['cases']:
            with self.subTest(input=case['input']):
                value = case['input']
                result = subprocess.run([str(binary), value], capture_output=True, text=True, encoding='utf-8')
                self.assertEqual(result.returncode, 0, result.stderr)
                expected = []
                for line in case['raw_stdout'].splitlines():
                    parts = line.split('\t')
                    if parts[0] == 'END':
                        continue  # final stream offset is not part of this API
                    expected.append((*map(int, parts[:3]), base64.b64decode(parts[3]).decode()))
                got = []
                for line in result.stdout.splitlines():
                    parts = line.split('\t', 3)
                    got.append((*map(int, parts[:3]), parts[3]))
                self.assertEqual(got, expected)

if __name__ == '__main__':
    unittest.main()
