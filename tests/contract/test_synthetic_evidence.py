"""Independently recheck retained synthetic pilot inputs and original HTTP paths."""
import hashlib
import json
from pathlib import Path
import sys
import unittest

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))
from build_synthetic_corpus import content, specification
from indexer_evidence import verify_candidate_coverage, verify_path_coverage


class SyntheticEvidence(unittest.TestCase):
    def test_indexer_log_references_exist_and_match_recorded_hashes(self):
        base=ROOT/'tests/evidence/synthetic-capacity-pilot-2026-11-01'
        receipt=json.loads((base/'original-indexer-log-check.json').read_text())
        report=json.loads((base/'report.json').read_text())
        self.assertEqual(receipt,report['original_log_check'])
        for stream in receipt['streams'].values():
            data=(base/stream['file']).read_bytes()
            self.assertEqual(len(data),stream['bytes'])
            self.assertEqual(hashlib.sha256(data).hexdigest(),stream['sha256'])
        earlier=base/'earlier-blocked'
        older=json.loads((earlier/'index-report.json').read_text())['original_log_check']
        for stream in older['streams'].values():
            data=(earlier/stream['file']).read_bytes()
            self.assertEqual(len(data),stream['bytes'])
            self.assertEqual(hashlib.sha256(data).hexdigest(),stream['sha256'])

    def test_recorded_candidate_contents_and_original_paths(self):
        base=ROOT/'tests/evidence/synthetic-capacity-pilot-2026-11-01'
        manifest=json.loads((base/'synthetic-manifest.json').read_text())
        self.assertEqual(manifest,specification(manifest['projects'],manifest['files_per_project'],manifest['bytes_per_file']))
        expected={}
        for item in manifest['files']:
            project=int(item['path'][7:9]); number=int(Path(item['path']).stem[9:])
            expected['/'+item['path']]=content(project,number,manifest['bytes_per_file']).decode()
        raw=(base/'candidate-index.json').read_bytes();report=json.loads((base/'report.json').read_text())
        self.assertEqual(hashlib.sha256(raw).hexdigest(),report['candidate_index_sha256'])
        self.assertTrue(verify_candidate_coverage(expected,json.loads(raw))['complete'])
        responses={p.name[len('response-'):-len('.json')]:json.loads(p.read_text()) for p in base.glob('response-*.json')}
        entries={'repositories':[{'path':str(Path(x['path']).parent),'file':Path(x['path']).name} for x in manifest['files']]}
        self.assertTrue(verify_path_coverage(entries,responses)['complete'])
        coverage=json.loads((base/'coverage.json').read_text())
        self.assertEqual(coverage['original_index_tree_before'],coverage['original_index_tree_after'])
        self.assertEqual(coverage['status'],'MATCHED_ALL_SYNTHETIC_PATHS')

if __name__=='__main__':unittest.main()
