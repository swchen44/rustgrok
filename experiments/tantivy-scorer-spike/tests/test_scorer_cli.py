"""Public interface checks for the Tantivy G1 scorer candidate."""
import json
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class ScorerProbe(unittest.TestCase):
    def test_preserved_input_replays_recorded_candidate_pages(self):
        repo=ROOT.parents[1]
        evidence=repo/'tests/evidence/grouped-single-term-pages-2026-11-01/comparison.json'
        docs=repo/'tests/evidence/lucene-single-term-2026-11-01/candidate-documents.json'
        recorded=json.loads(evidence.read_text())
        self.assertEqual(hashlib.sha256(docs.read_bytes()).hexdigest(),recorded['candidate_index_input_sha256'])
        self.assertEqual(len(recorded['cases']),12)
        for case in recorded['cases']:
            cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(docs),'--field','full','--query','main','--lucene-single-term','--start',str(case['start']),'--maxresults',str(case['maxresults'])]
            for project in case['projects']: cmd.extend(['--project',project])
            actual=json.loads(subprocess.check_output(cmd,text=True))
            self.assertEqual((actual['resultCount'],actual['startDocument'],actual['endDocument'],[hit['path'] for hit in actual['hits']]),
                             (case['candidate_count'],case['candidate_start'],case['candidate_end'],case['candidate_ordered_paths']))
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
    def test_single_term_score_bits_match_original_public_corpus(self):
        repo=ROOT.parents[1];fixture=repo/'tests/fixtures/public-source'
        sources=[('alpha','component-c','c/main.c'),('alpha','component-js','javascript/function.js'),('alpha','component-c/nested-js','javascript/function.js'),('beta','component-c','c/main.c'),('beta','component-py','python/main.py.fixture')]
        docs=[]
        for project,component,name in sources:
            display=name.removesuffix('.fixture').split('/')[-1]
            docs.append({'path':f'/{project}/{component}/{display}','project':project,'content':(fixture/name).read_text(),'definitions':[]})
        import re,struct
        raw=(repo/'tests/evidence/search-spike-2026-11-01/score-other-cases/full_both.txt').read_text()
        expected={path:struct.pack('>f',float.fromhex(score)).hex() for score,path in re.findall(r'^doc=\d+ score=(\S+) fullpath=\S+ path=(\S+)',raw,re.M)}
        self.assertEqual(len(expected),5)
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'documents.json';path.write_text(json.dumps(docs))
            cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(path),'--field','full','--query','main','--project','alpha','--project','beta','--lucene-single-term']
            result=json.loads(subprocess.check_output(cmd,text=True))
            self.assertEqual({hit['path']:hit['score_bits'] for hit in result['hits']},expected)
    def test_sparse_field_document_count_and_score_match_original(self):
        repo=ROOT.parents[1];source=repo/'tests/evidence/sparse-single-term-2026-11-01/source'
        docs=[{'path':f'/alpha/{name}','project':'alpha','content':(source/name).read_text(),'definitions':[]} for name in ('a.c','b.c','c.c')]
        import re,struct
        raw=(source.parent/'oracle-score.txt').read_text()
        score=re.search(r'^doc=\d+ score=(\S+).*path=(\S+)',raw,re.M)
        expected=struct.pack('>f',float.fromhex(score.group(1))).hex()
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'documents.json';path.write_text(json.dumps(docs))
            cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(path),'--field','full','--query','main','--project','alpha','--lucene-single-term']
            actual=json.loads(subprocess.check_output(cmd,text=True))
            self.assertEqual(actual['field_document_count'],2)
            self.assertEqual(actual['term_document_frequency'],1)
            self.assertEqual([(x['path'],x['score_bits']) for x in actual['hits']],[(score.group(2),expected)])
    def test_single_term_scores_all_matches_before_ranking(self):
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'documents.json'
            docs=[{'path':f'/alpha/{i}.c','project':'alpha','content':'main filler filler','definitions':[]} for i in range(200)]
            docs[-1]['content']='main main main main filler'
            path.write_text(json.dumps(docs))
            cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(path),'--field','full','--query','main','--project','alpha','--lucene-single-term']
            data=json.loads(subprocess.check_output(cmd,text=True))
            self.assertEqual(data['resultCount'],200)
            self.assertEqual(len(data['hits']),200)
            self.assertEqual(data['hits'][0]['path'],'/alpha/199.c')
    def test_multi_project_term_pages_match_original_across_project_order(self):
        repo=ROOT.parents[1];fixture=repo/'tests/fixtures/public-source';evidence=repo/'tests/evidence/project-order-original-2026-11-01'
        sources=[('alpha','component-c','c/main.c'),('alpha','component-js','javascript/function.js'),('alpha','component-c/nested-js','javascript/function.js'),('beta','component-c','c/main.c'),('beta','component-py','python/main.py.fixture')]
        docs=[]
        for project,component,name in reversed(sources):
            display=name.removesuffix('.fixture').split('/')[-1]
            docs.append({'path':f'/{project}/{component}/{display}','project':project,'content':(fixture/name).read_text(),'definitions':[]})
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'documents.json';path.write_text(json.dumps(docs))
            for first,second in (('alpha','beta'),('beta','alpha')):
                for start in range(3):
                    raw=json.loads((evidence/f'full-{first}-start{start}-max1-r0.json').read_text())
                    cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(path),'--field','full','--query','main','--project',first,'--project',second,'--lucene-single-term','--start',str(start),'--maxresults','1']
                    actual=json.loads(subprocess.check_output(cmd,text=True))
                    self.assertEqual(actual['resultCount'],raw['resultCount'])
                    self.assertEqual([hit['path'] for hit in actual['hits']],list(raw['results']),(first,start))
    def test_single_term_page_boundaries_match_original(self):
        repo=ROOT.parents[1];fixture=repo/'tests/fixtures/public-source';evidence=repo/'tests/evidence/project-order-boundaries-2026-11-01'
        sources=[('alpha','component-c','c/main.c'),('alpha','component-js','javascript/function.js'),('alpha','component-c/nested-js','javascript/function.js'),('beta','component-c','c/main.c'),('beta','component-py','python/main.py.fixture')]
        docs=[]
        for project,component,name in sources:
            display=name.removesuffix('.fixture').split('/')[-1]
            docs.append({'path':f'/{project}/{component}/{display}','project':project,'content':(fixture/name).read_text(),'definitions':[]})
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'documents.json';path.write_text(json.dumps(docs))
            for start,maximum in ((0,0),(100,1)):
                raw=json.loads((evidence/f'full-alpha-start{start}-max{maximum}-r0.json').read_text())
                cmd=['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','--documents',str(path),'--field','full','--query','main','--project','alpha','--project','beta','--lucene-single-term','--start',str(start),'--maxresults',str(maximum)]
                actual=json.loads(subprocess.check_output(cmd,text=True))
                self.assertEqual((actual['resultCount'],actual['startDocument'],actual['endDocument'],[h['path'] for h in actual['hits']]),(raw['resultCount'],raw['startDocument'],raw['endDocument'],list(raw['results'])))
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
