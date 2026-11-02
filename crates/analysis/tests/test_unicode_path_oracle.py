"""Compare supported Unicode token streams and true byte spans with the original."""
import base64
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[3]
CRATE = ROOT / 'crates/analysis'
EVIDENCE = ROOT / 'tests/evidence/unicode-path-2026-11-03/oracle.json'
SOURCES = [
    ['專案', '目錄', '程式', '.', 'c'], ['p', '🦀', '.', 'C'],
    ['🦀😀', '標題', '.', 'js'], ['p', 'İSTANBUL', '.', 'c'], ['p', 'İ', '.', 'c'],
    ['Σ'], ['AΣ'], ['AΣA'], ['ΟΣ'], ['ΟΣΑ'], ['AΣ', '.', 'B'],
    None, None, None, None, ['ΣA'], ['AΣİ'],
    ['專案', '🦀', '.', 'İ'], ['alpha', '🦀', '.', 'c'], ['beta', 'İ', '.', 'js'],
    ['繁體中文', 'AΣ', '😀', '.', 'c'], ['AΣ'], ['AΣ', 'A'], 'ENTIRE_INPUT',
]

class UnicodePathOracle(unittest.TestCase):
    def test_supported_streams_match_original_and_unsupported_contexts_fail_closed(self):
        subprocess.run(['cargo', 'build', '--locked', '--offline', '--quiet', '--manifest-path',
                        str(CRATE / 'Cargo.toml'), '--example', 'path_tokens'], check=True)
        binary = CRATE / 'target/debug/examples/path_tokens'
        data = json.loads(EVIDENCE.read_text(encoding='utf-8'))
        self.assertEqual(len(data['cases']), len(SOURCES))
        for case, expected_source in zip(data['cases'], SOURCES):
            with self.subTest(id=case['id']):
                value = case['input']
                self.assertEqual(hashlib.sha256(value.encode()).hexdigest(), case['input_sha256'])
                self.assertEqual(hashlib.sha256(case['raw_stdout'].encode()).hexdigest(), case['stdout_sha256'])
                result = subprocess.run([str(binary), '--stdin', '--spans'], input=value, capture_output=True, text=True, encoding='utf-8')
                if expected_source is None:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn('unsupported byte at offset', result.stderr)
                    self.assertEqual(result.stdout, '')
                    continue
                self.assertEqual(result.returncode, 0, result.stderr)
                original = []
                for line in case['raw_stdout'].splitlines():
                    parts = line.split('\t')
                    if parts[0] != 'END':
                        original.append((*map(int, parts[:3]), base64.b64decode(parts[3]).decode()))
                actual = []
                sources = []
                for line in result.stdout.splitlines():
                    parts = line.split('\t', 5)
                    position, start, end, byte_start, byte_end = map(int, parts[:5])
                    actual.append((position, start, end, parts[5]))
                    sources.append(value.encode()[byte_start:byte_end].decode())
                self.assertEqual(actual, original)
                self.assertEqual(sources, [value] if expected_source == 'ENTIRE_INPUT' else expected_source)

class SigmaContexts(unittest.TestCase):
    def test_all_3600_bounded_sigma_contexts_match_original(self):
        evidence = EVIDENCE.parent
        receipt = json.loads((evidence / 'sigma-contexts-receipt.json').read_text(encoding='utf-8'))
        raw = (evidence / 'sigma-contexts.stdout').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), receipt['raw_stdout_sha256'])
        self.assertEqual(hashlib.sha256((evidence / 'SigmaContextOracle.java').read_bytes()).hexdigest(),
                         receipt['wrapper_source_sha256'])
        rows = [line.split('\t') for line in raw.decode().splitlines()]
        self.assertEqual(len(rows), 3600)
        inputs = [base64.b64decode(row[0]).decode() for row in rows]
        expected = [f'{value}\t{row[1]}\t{row[2]}\t{row[3]}\t{base64.b64decode(row[4]).decode()}'
                    for value, row in zip(inputs, rows)]
        subprocess.run(['cargo', 'build', '--locked', '--offline', '--quiet', '--manifest-path',
                        str(CRATE / 'Cargo.toml'), '--example', 'sigma_contexts'], check=True)
        binary = CRATE / 'target/debug/examples/sigma_contexts'
        result = subprocess.run([str(binary)], input='\n'.join(inputs) + '\n',
                                capture_output=True, text=True, encoding='utf-8', check=True)
        self.assertEqual(result.stdout.splitlines(), expected)

if __name__ == '__main__':
    unittest.main()
