"""Verify the recorded bounded source-scanner comparison and raw streams."""
import hashlib
import json
import subprocess
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'tests/evidence/source-scanner-2026-11-03'
ORIGINAL = ROOT / 'tests/evidence/live-path-no-hit-verified-2026-11-01'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class SourceScannerEvidence(unittest.TestCase):
    def test_receipt_input_and_original_sources(self):
        receipt = json.loads((EVIDENCE / 'receipt.json').read_text())
        self.assertEqual(receipt['exit_code'], 0)
        inventory = {entry['path']: entry['sha256'] for entry in json.loads((ROOT / 'upstream/opengrok-source-tree.json').read_text())['files']}
        for path, sha in receipt['upstream_sha256'].items():
            self.assertEqual(inventory[path], sha)
        for path, sha in receipt['files'].items():
            raw = (ROOT / path).read_bytes()
            if hashlib.sha256(raw).hexdigest() != sha:
                raw = subprocess.check_output(["git", "show", f"6f55e36f1a3b3cb7962cc1a04f3b9f4e8b18dac0:{path}"], cwd=ROOT)
            self.assertEqual(hashlib.sha256(raw).hexdigest(), sha, path)
        for stream in ('stdout', 'stderr'):
            self.assertEqual(digest(EVIDENCE / f'scanner.{stream}.txt'), receipt[f'{stream}_sha256'])
        self.assertIn('4 passed; 0 failed', (EVIDENCE / 'scanner.stdout.txt').read_text())

    def test_original_raw_path_projection_and_git_filter(self):
        results = set()
        for name in ('coverage-function.js.body', 'coverage-main.c.body', 'coverage-main.py.body'):
            results.update(json.loads((ORIGINAL / name).read_text())['results'])
        self.assertEqual(results, {'/alpha/component-c/main.c', '/alpha/component-c/nested-js/function.js', '/alpha/component-js/function.js', '/beta/component-c/main.c', '/beta/component-py/main.py'})
        xml = ET.parse(ORIGINAL / 'configuration.xml')
        for kind in ('ignoredDirs', 'ignoredFiles'):
            element = xml.find(f'.//void[@property="{kind}"]')
            self.assertIsNotNone(element)
            self.assertIn('.git', [item.text for item in element.findall('.//string')])


if __name__ == '__main__':
    unittest.main()
