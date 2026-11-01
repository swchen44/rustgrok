"""Benchmark index evidence must retain logs and reject known source-file failures."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))


class IndexerEvidence(unittest.TestCase):
    def test_exit_zero_file_failures_block_and_raw_logs_are_preserved(self):
        from indexer_evidence import record_indexer_logs
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); fresh = base / 'fresh'; dest = base / 'out'
            fresh.mkdir(); dest.mkdir()
            stderr = (ROOT/'tests/evidence/long-token-indexer-2026-11-01/split.indexer.stderr.txt').read_bytes()
            (fresh/'index.stderr.txt').write_bytes(stderr)
            (fresh/'index.stdout.txt').write_bytes(b'')
            report = record_indexer_logs(fresh, dest)
            self.assertEqual(report['status'], 'BLOCKED_KNOWN_FILE_FAILURE')
            self.assertEqual((dest/'original-indexer.stderr.txt').read_bytes(),stderr)
            self.assertEqual((dest/'original-indexer.stdout.txt').read_bytes(),b'')
            self.assertTrue(any('addFile' in x for x in report['indicators']))

    def test_missing_success_marker_is_not_reported_as_verified(self):
        from indexer_evidence import record_indexer_logs
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); fresh = base/'fresh'; dest = base/'out'; fresh.mkdir(); dest.mkdir()
            (fresh/'index.stderr.txt').write_bytes(b'Indexing started\n')
            (fresh/'index.stdout.txt').write_bytes(b'')
            self.assertEqual(record_indexer_logs(fresh,dest)['status'],'BLOCKED_MISSING_COMPLETION_MARKER')

    def test_observed_paths_must_cover_each_source_exactly_once(self):
        from indexer_evidence import verify_path_coverage
        manifest = {'repositories':[{'path':'alpha/a','file':'main.c'},{'path':'beta/b','file':'main.py'}]}
        good = {'main.c':{'resultCount':1,'results':{'/alpha/a/main.c':[]}}, 'main.py':{'resultCount':1,'results':{'/beta/b/main.py':[]}}}
        self.assertTrue(verify_path_coverage(manifest,good)['complete'])
        good['main.py']['results']={}
        self.assertFalse(verify_path_coverage(manifest,good)['complete'])

    def test_candidate_must_have_every_path_and_matching_source(self):
        from indexer_evidence import verify_candidate_coverage
        expected={'/alpha/a.c':'source a','/beta/b.c':'source b'}
        docs=[{'path':'/alpha/a.c','content':'source a'},{'path':'/beta/b.c','content':'source b'}]
        self.assertTrue(verify_candidate_coverage(expected,docs)['complete'])
        docs[1]['content']='wrong'
        self.assertFalse(verify_candidate_coverage(expected,docs)['complete'])
        docs[1]={'path':'/alpha/a.c','content':'source a'}
        self.assertFalse(verify_candidate_coverage(expected,docs)['complete'])

    def test_saved_candidate_index_is_the_recorded_public_source_index(self):
        fixture=ROOT/'tests/fixtures/public-source'
        names={'main.c':'c/main.c','main.py':'python/main.py.fixture','function.js':'javascript/function.js'}
        manifest=json.loads((fixture/'expected-corpus-manifest.json').read_text())
        paths={'/'+r['path']+'/'+r['file'] for r in manifest['repositories']}
        for case in ('no-hit','single-hit'):
            with self.subTest(case=case):
                evidence=ROOT/f'tests/evidence/live-path-{case}-verified-2026-11-01'
                raw=(evidence/'candidate-index.json').read_bytes()
                metadata=json.loads((evidence/'metadata.json').read_text())
                docs=json.loads(raw)
                self.assertEqual(hashlib.sha256(raw).hexdigest(),metadata['candidate_index_sha256'])
                self.assertEqual(len(docs),5)
                self.assertEqual({d['path'] for d in docs},paths)
                for d in docs:
                    self.assertEqual(d['content'],(fixture/names[Path(d['path']).name]).read_text())

if __name__ == '__main__': unittest.main()
