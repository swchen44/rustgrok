"""Contract tests for the full-output HTTP timing gate."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

GATE = Path(__file__).resolve().parents[2] / 'tools/search_benchmark_gate.py'

class SearchBenchmarkGate(unittest.TestCase):
    def run_gate(self, oracle, candidate, oracle_status=200, candidate_status=200):
        with tempfile.TemporaryDirectory() as tmp:
            a, b = Path(tmp) / 'a.json', Path(tmp) / 'b.json'
            a.write_text(json.dumps(oracle)); b.write_text(json.dumps(candidate))
            p = subprocess.run([sys.executable, str(GATE), '--oracle', str(a), '--candidate', str(b),
                                '--oracle-status', str(oracle_status), '--candidate-status', str(candidate_status)],
                               capture_output=True, text=True)
            return p, json.loads(p.stdout)

    def sample(self):
        return {'time': 42, 'resultCount': 2, 'results': {
            '/a': [{'line':'<b>word</b>', 'lineNumber':'1', 'tag':'function'}],
            '/b': [{'line':'word', 'lineNumber':'2', 'tag':None}]},
            'startDocument': 0, 'endDocument': 1}

    def test_variable_time_is_the_only_allowed_value_difference(self):
        a = self.sample(); b = self.sample(); b['time'] = 900
        p, report = self.run_gate(a, b)
        self.assertEqual(p.returncode, 0)
        self.assertEqual(report['status'], 'BODY_MATCH_PRECHECK_ONLY')
        self.assertEqual(report['normalization'], ['top-level time value'])

    def test_path_order_difference_blocks_timing_even_if_json_objects_match(self):
        a = self.sample(); b = self.sample(); b['results'] = dict(reversed(list(b['results'].items())))
        p, report = self.run_gate(a, b)
        self.assertNotEqual(p.returncode, 0)
        self.assertEqual(report['status'], 'BLOCKED_OUTPUT_DIFFERENCE')

    def test_missing_snippet_or_tag_blocks_timing(self):
        for field in ('line', 'tag'):
            a = self.sample(); b = self.sample(); del b['results']['/a'][0][field]
            p, report = self.run_gate(a, b)
            self.assertNotEqual(p.returncode, 0, field)
            self.assertEqual(report['status'], 'BLOCKED_OUTPUT_DIFFERENCE')

    def test_missing_time_field_is_a_schema_difference(self):
        a = self.sample(); b = self.sample(); del b['time']
        p, report = self.run_gate(a, b)
        self.assertNotEqual(p.returncode, 0)
        self.assertEqual(report['status'], 'BLOCKED_OUTPUT_DIFFERENCE')

    def test_equal_error_responses_never_qualify(self):
        a = {'error': 'bad request'}
        p, report = self.run_gate(a, a, 400, 400)
        self.assertNotEqual(p.returncode, 0)
        self.assertEqual(report['status'], 'BLOCKED_HTTP_STATUS')

if __name__ == '__main__':
    unittest.main()
