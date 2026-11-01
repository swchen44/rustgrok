"""Public interface checks for the Tantivy G1 scorer candidate."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class ScorerProbe(unittest.TestCase):
    def test_underscore_identifier_results_match_original_on_public_corpus(self):
        repo = ROOT.parents[1]
        fixture = repo/'tests/fixtures/public-source'
        sources = [('alpha','component-c','c/main.c'), ('alpha','component-js','javascript/function.js'), ('alpha','component-c/nested-js','javascript/function.js'), ('beta','component-c','c/main.c'), ('beta','component-py','python/main.py.fixture')]
        docs=[]
        for project,component,name in sources:
            display = name.removesuffix('.fixture').split('/')[-1]
            docs.append({'path':f'/{project}/{component}/{display}','project':project,'content':(fixture/name).read_text(),'definitions':[]})
        raw=(repo/'tests/evidence/full-token-diagnostic-2026-11-01/oracle-query.txt').read_text()
        expected=[line.split(' path=',1)[1] for line in raw.splitlines() if line.startswith('doc=')]
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'documents.json';path.write_text(json.dumps(docs))
            cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(path),'--field','full','--query','__main__','--project','alpha','--project','beta']
            actual=json.loads(subprocess.check_output(cmd,text=True))
            self.assertEqual([hit['path'] for hit in actual['hits']],expected)
    def test_unpaired_utf16_truncation_boundary_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'documents.json'
            path.write_text(json.dumps([{'path':'/alpha/example.c','project':'alpha','content':'a'*32765+'\U00010400','definitions':[]}]))
            cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(path),'--field','full','--query','a','--project','alpha']
            result=subprocess.run(cmd,capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('unsupported UTF-16 surrogate split',result.stderr)
    def test_original_oversized_utf8_term_case_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'documents.json'
            path.write_text(json.dumps([{'path':'/alpha/example.c','project':'alpha','content':'中'*11000,'definitions':[]}]))
            cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(path),'--field','full','--query','a','--project','alpha']
            result=subprocess.run(cmd,capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('unsupported token exceeding Lucene byte limit',result.stderr)
    def test_selected_project_scores_repeated_term_ahead(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'documents.json'
            path.write_text(json.dumps([
                {'path': '/alpha/a.c', 'project': 'alpha', 'content': 'main x padding padding', 'definitions': []},
                {'path': '/alpha/b.c', 'project': 'alpha', 'content': 'main main padding padding', 'definitions': []},
                {'path': '/beta/c.c', 'project': 'beta', 'content': 'main main main main', 'definitions': []},
            ]))
            cmd = ['cargo', 'run', '--release', '--locked', '--offline', '--quiet', '--manifest-path', str(ROOT / 'Cargo.toml'), '--', '--documents', str(path), '--field', 'full', '--query', 'main', '--project', 'alpha']
            data = json.loads(subprocess.check_output(cmd, text=True))
            self.assertEqual([h['path'] for h in data['hits']], ['/alpha/b.c', '/alpha/a.c'])
            self.assertGreater(data['hits'][0]['score'], data['hits'][1]['score'])
            self.assertEqual(data['indexed_documents'], 2)

if __name__ == '__main__':
    unittest.main()
