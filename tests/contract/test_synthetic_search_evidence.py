"""Independently verify retained responses without relaxing their recorded order."""
import hashlib
import json
import math
from pathlib import Path
import sys
import unittest

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))
from build_synthetic_corpus import content, specification
from indexer_evidence import verify_candidate_coverage, verify_path_coverage
from search_benchmark_gate import comparison, load_response


class SyntheticSearchEvidence(unittest.TestCase):
    def test_inputs_and_original_40_paths(self):
        base=ROOT/'tests/evidence/synthetic-search-gate-2026-11-01'
        source=ROOT/'tests/evidence/synthetic-capacity-pilot-2026-11-01'
        manifest=json.loads((source/'synthetic-manifest.json').read_text())
        self.assertEqual(manifest,specification(manifest['projects'],manifest['files_per_project'],manifest['bytes_per_file']))
        expected={}
        for item in manifest['files']:
            project=int(item['path'][7:9]);number=int(Path(item['path']).stem[9:])
            expected['/'+item['path']]=content(project,number,manifest['bytes_per_file']).decode()
        raw=(base/'candidate-index.json').read_bytes();original=json.loads((source/'report.json').read_text())
        self.assertEqual(hashlib.sha256(raw).hexdigest(),original['candidate_index_sha256'])
        self.assertTrue(verify_candidate_coverage(expected,json.loads(raw))['complete'])
        responses={p.name[len('response-'):-len('.json')]:json.loads(p.read_text()) for p in base.glob('response-*.json')}
        entries={'repositories':[{'path':str(Path(x['path']).parent),'file':Path(x['path']).name} for x in manifest['files']]}
        self.assertTrue(verify_path_coverage(entries,responses)['complete'])
        coverage=json.loads((base/'coverage.json').read_text());proof=json.loads((source/'coverage.json').read_text())
        self.assertEqual(coverage['original_index_tree_before'],proof['original_index_tree_before'])
        self.assertEqual(coverage['original_index_tree_after'],proof['original_index_tree_after'])
        logs=json.loads((base/'original-indexer-log-check.json').read_text())
        self.assertEqual(logs['status'],'NO_KNOWN_FILE_FAILURE_INDICATORS')
        self.assertEqual(logs,original['original_log_check'])
        for item in logs['streams'].values():self.assertEqual(hashlib.sha256((base/item['file']).read_bytes()).hexdigest(),item['sha256'])

    def test_disk_guard_observations_and_failed_rerun_receipt(self):
        base=ROOT/'tests/evidence/synthetic-search-gate-2026-11-01'
        watch=json.loads((base/'disk-watch.json').read_text())
        self.assertFalse(watch['breached'])
        self.assertGreaterEqual(min(x['free_bytes'] for x in watch['samples']),watch['reserve_bytes'])
        blocked=json.loads((base/'v2-guard-blocked.json').read_text())
        self.assertEqual(blocked['status'],'BLOCKED_BEFORE_OUTPUT_CREATION')
        self.assertLess(blocked['observed_free_bytes'],blocked['reserve_bytes'])
        self.assertFalse(blocked['output_existed_at_evidence_capture'])

    def test_matched_bodies_and_percentiles_and_blocked_order(self):
        base=ROOT/'tests/evidence/synthetic-search-gate-2026-11-01'
        for case in ('single-hit','no-hit'):
            directory=base/('benchmark-'+case);report=json.loads((directory/'report.json').read_text())
            self.assertEqual(report['status'],'COMPLETED_RESTRICTED_CASE')
            self.assertEqual(report['timed_samples'],200)
            reference=load_response(directory/report['preflight'][0]['body'])[1]
            for sample in report['samples']:
                self.assertEqual(sample['status'],200);self.assertIsNone(sample['error'])
                self.assertTrue(comparison(reference,load_response(directory/sample['body'])[1]))
            for load in (1,5):
                for endpoint,name in enumerate(('oracle','candidate')):
                    values=sorted(s['elapsed_ns'] for s in report['samples'] if s['concurrency']==load and s['endpoint']==endpoint)
                    self.assertEqual(len(values),50)
                    summary=report['summary'][f'{name}_c{load}'];self.assertEqual(summary['errors'],0)
                    for percentile in (50,95,99):self.assertEqual(summary[f'p{percentile}_ns'],values[math.ceil(percentile*len(values)/100)-1])
        directory=base/'benchmark-multiple-hit';report=json.loads((directory/'report.json').read_text())
        self.assertEqual((report['status'],report['timed_samples']),('BLOCKED_OUTPUT_DIFFERENCE',0))
        original=load_response(directory/report['preflight'][0]['body'])[1];candidate=load_response(directory/report['preflight'][1]['body'])[1]
        self.assertEqual(original['resultCount'],10);self.assertEqual(candidate['resultCount'],10)
        self.assertEqual(set(original['results']),set(candidate['results']))
        self.assertNotEqual(list(original['results']),list(candidate['results']))

if __name__=='__main__':unittest.main()
