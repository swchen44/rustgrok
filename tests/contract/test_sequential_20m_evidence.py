"""Recheck the 20 MiB synthetic run's raw inputs, gates and latency arithmetic."""
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
from sequential_http_measure import load_eligibility


class Sequential20MiBEvidence(unittest.TestCase):
    def test_inputs_original_paths_and_stopped_groups(self):
        base=ROOT/'tests/evidence/synthetic-20m-sequential-2026-11-01';probe=base/'probe'
        manifest=json.loads((probe/'synthetic-manifest.json').read_text())
        self.assertEqual(manifest,specification(manifest['projects'],manifest['files_per_project'],manifest['bytes_per_file']))
        self.assertEqual((manifest['projects'],len(manifest['files']),manifest['source_bytes']),(10,80,20971520))
        expected={}
        for item in manifest['files']:
            project=int(item['path'][7:9]);number=int(Path(item['path']).stem[9:])
            expected['/'+item['path']]=content(project,number,manifest['bytes_per_file']).decode()
        raw=(probe/'candidate-index.json').read_bytes();report=json.loads((probe/'report.json').read_text())
        self.assertEqual(hashlib.sha256(raw).hexdigest(),report['candidate_index_sha256'])
        analysis=json.loads((base/'source-analysis.json').read_text())
        self.assertEqual(analysis['candidate_binary_sha256'],report['candidate_binary_sha256'])
        for path,digest in analysis['source_files'].items():self.assertEqual(hashlib.sha256((ROOT/path).read_bytes()).hexdigest(),digest)
        self.assertTrue(verify_candidate_coverage(expected,json.loads(raw))['complete'])
        self.assertTrue(report['original_process']['group_stopped']);self.assertTrue(report['candidate_process']['group_stopped'])
        for item in report['original_log_check']['streams'].values():self.assertEqual(hashlib.sha256((probe/item['file']).read_bytes()).hexdigest(),item['sha256'])
        original=base/'original-preflight';coverage=json.loads((original/'coverage.json').read_text())
        self.assertEqual(coverage['original_index_tree_before'],coverage['original_index_tree_after'])
        self.assertTrue(all(coverage['process_groups_stopped'].values()))
        bodies={p.name[len('response-'):-len('.json')]:json.loads(p.read_text()) for p in original.glob('response-*.json')}
        entries={'repositories':[{'path':str(Path(x['path']).parent),'file':Path(x['path']).name} for x in manifest['files']]}
        self.assertTrue(verify_path_coverage(entries,bodies)['complete'])
        self.assertTrue(json.loads((base/'candidate-preflight/disk-watch.json').read_text())['candidate_group_stopped'])
        self.assertTrue(all(json.loads((base/'oracle-measure/coverage.json').read_text())['process_groups_stopped'].values()))
        candidate_report=json.loads((base/'candidate-measure/report.json').read_text())
        self.assertEqual(candidate_report['status'],'BODY_ONLY_DIAGNOSTIC_NOT_TIMED')
        self.assertEqual(len(candidate_report['measurements']),2)
        self.assertTrue(all(candidate_report['process_groups_stopped'].values()))

    def test_preflight_raw_samples_percentiles_and_blocked_multi(self):
        base=ROOT/'tests/evidence/synthetic-20m-sequential-2026-11-01';proof=base/'candidate-preflight'
        proof_hash=hashlib.sha256((proof/'sequential-preflight.json').read_bytes()).hexdigest()
        for case in ('single-hit','no-hit'):
            eligible=load_eligibility(proof,case);reference=eligible['oracle']['followup']
            for role in ('oracle','candidate'):
                directory=base/(role+'-measure')/(f'measure-{role}-{case}')
                report=json.loads((directory/'report.json').read_text())
                self.assertEqual((report['status'],report['role'],report['timed_samples']),('COMPLETED_RESTRICTED_ENDPOINT',role,100))
                self.assertEqual(report['proof_sha256'],proof_hash);self.assertTrue(report['unique_session_cookies_per_load'])
                for sample in report['samples']:
                    self.assertEqual(sample['status'],200);self.assertIsNone(sample['error']);self.assertFalse(sample['cookie_present'])
                    self.assertEqual(sample['headers'],reference['headers'])
                    self.assertTrue(comparison(reference['body'],load_response(directory/sample['body'])[1]))
                for load in (1,5):
                    values=sorted(x['elapsed_ns'] for x in report['samples'] if x['concurrency']==load)
                    self.assertEqual(len(values),50);summary=report['summary'][str(load)];self.assertEqual(summary['errors'],0)
                    for p in (50,95,99):self.assertEqual(summary[f'p{p}_ns'],values[math.ceil(p*len(values)/100)-1])
        cross=json.loads((proof/'report.json').read_text())['cases'];self.assertEqual(len(cross),8)
        for case in cross:
            original=load_response(proof/case['oracle_body'])[1];candidate=load_response(proof/case['candidate_body'])[1]
            self.assertEqual((original['resultCount'],candidate['resultCount']),(10,10))
            self.assertEqual(set(original['results']),set(candidate['results']))
            self.assertNotEqual(list(original['results']),list(candidate['results']))
            self.assertFalse(comparison(original,candidate))

if __name__=='__main__':unittest.main()
