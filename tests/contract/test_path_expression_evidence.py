"""Verify retained original HTTP bodies back the finite Rust query fixture."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / 'tests/evidence/path-query-2026-11-03'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PathExpressionEvidence(unittest.TestCase):
    def test_recorded_sources_and_original_http_projection(self):
        source = json.loads((EVIDENCE / 'source-provenance.json').read_text())
        lock = json.loads((ROOT / 'upstream/opengrok-source-tree.json').read_text())
        self.assertEqual(source['upstream_commit'], lock['commit'])
        locked = {item['path']: item['sha256'] for item in lock['files']}
        for item in source['files']:
            self.assertEqual(item['sha256'], locked[item['path']])
        blocked = {'alpha OR beta AND gamma', 'alpha OR NOT beta', 'alpha +beta',
                   'alpha-beta', 'alpha+beta', 'NOT NOT alpha'}
        projected = []
        for folder, count in ((EVIDENCE, 16), (EVIDENCE / 'differential', 31)):
            receipt = json.loads((folder / 'receipt.json').read_text())
            self.assertEqual(receipt['upstream_commit'], lock['commit'])
            for name in ('parser_stdout', 'parser_stderr', 'index_stdout', 'index_stderr'):
                stem, stream = name.split('_')
                file = folder / f'{"parse" if stem == "parser" else stem}.{stream}.txt'
                self.assertEqual(digest(file), receipt[name + '_sha256'])
            for case in receipt['cases']:
                self.assertEqual(digest(folder / case['body']), case['sha256'])
            for case, expected in zip(receipt['cases'][-5:], receipt['synthetic_files']):
                self.assertEqual(case['status'], 200)
                body = json.loads((folder / case['body']).read_text())
                self.assertEqual(body['resultCount'], 1)
                self.assertEqual(set(body['results']), {'/demo/' + expected})
            for case in receipt['cases'][:count]:
                if case['query'] in blocked:
                    continue
                self.assertEqual(case['status'], 200)
                body = json.loads((folder / case['body']).read_text())
                self.assertEqual(body['resultCount'], len(body['results']))
                projected.append(case['query'] + '\t' + '|'.join(sorted(body['results'])))
        self.assertEqual(len(projected), 41)
        self.assertEqual((EVIDENCE / 'oracle-accepted.tsv').read_text(), '\n'.join(projected) + '\n')


    def test_recorded_native_test_runs(self):
        receipt = json.loads((EVIDENCE / 'rust-test-receipt.json').read_text())
        self.assertEqual({run['label'] for run in receipt['runs']}, {'path-index', 'host-default', 'host-hooks'})
        for run in receipt['runs']:
            self.assertEqual(run['exit_code'], 0)
            for stream in ('stdout', 'stderr'):
                self.assertEqual(digest(EVIDENCE / f"{run['label']}.{stream}.txt"), run[stream + '_sha256'])
        for path, expected in receipt['inputs'].items():
            current = ROOT / path
            if digest(current) == expected:
                continue
            # Historical source can be recovered from the commit that introduced this receipt.
            import subprocess
            commit = subprocess.check_output(['git', 'log', '-1', '--format=%H', '--diff-filter=A', '--',
                                              'tests/evidence/path-query-2026-11-03/rust-test-receipt.json'], cwd=ROOT, text=True).strip()
            self.assertTrue(commit, path)
            data = subprocess.check_output(['git', 'show', f'{commit}:{path}'], cwd=ROOT)
            self.assertEqual(hashlib.sha256(data).hexdigest(), expected, path)
        for label, counts in [('path-index', (21, 14, 4, 2)), ('host-default', (4, 15)), ('host-hooks', (6, 23))]:
            output = (EVIDENCE / f'{label}.stdout.txt').read_text()
            for count in counts:
                self.assertIn(f'{count} passed; 0 failed', output)


if __name__ == '__main__':
    unittest.main()
