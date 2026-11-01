"""Live benchmark rejects unequal responses before any timed samples."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest

ROOT = Path(__file__).resolve().parents[2]
class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = json.dumps({'time': 1, 'resultCount': 1, 'results': {'/a': [{'line': self.server.line, 'lineNumber': '', 'tag': None}]}, 'startDocument': 0, 'endDocument': 0}).encode()
        self.send_response(200); self.send_header('Content-Type', 'application/json');
        if getattr(self.server, 'cookie', False) and 'JSESSIONID=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA' not in self.headers.get('Cookie', ''): self.send_header('Set-Cookie', 'JSESSIONID=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA; Path=/source; HttpOnly')
        if getattr(self.server, 'extra_header', False): self.send_header('X-Feature', 'present')
        self.end_headers(); self.wfile.write(body)
    def log_message(self, *_): pass
class LiveBenchmark(unittest.TestCase):
    def test_matching_responses_produce_single_and_five_client_samples(self):
        servers = [ThreadingHTTPServer(('127.0.0.1', 0), Handler) for _ in range(2)]
        for server in servers:
            server.line = '...'; threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            with tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp)/'output'
                urls = ['http://127.0.0.1:%d/source/api/v1/search?path=a&start=0&maxresults=10' % s.server_port for s in servers]
                result = subprocess.run([sys.executable, str(ROOT/'tools/search_live_benchmark.py'), '--oracle', urls[0], '--candidate', urls[1], '--output', str(output), '--warmup', '1', '--samples', '5'], capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                report = json.loads((output/'report.json').read_text())
                self.assertEqual(report['status'], 'COMPLETED_RESTRICTED_CASE')
                self.assertEqual(report['timed_samples'], 20)
                self.assertEqual({entry['concurrency'] for entry in report['samples']}, {1, 5})
                self.assertTrue(all(entry['errors'] == 0 and entry['count'] == 5 for entry in report['summary'].values()))
        finally:
            for server in servers: server.shutdown(); server.server_close()
    def test_matched_session_creation_and_followup_can_run(self):
        servers = [ThreadingHTTPServer(('127.0.0.1', 0), Handler) for _ in range(2)]
        for server in servers:
            server.line = '...'; server.cookie = True; threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            with tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp)/'output'
                urls = ['http://127.0.0.1:%d/source/api/v1/search?path=a&start=0&maxresults=10' % s.server_port for s in servers]
                result = subprocess.run([sys.executable, str(ROOT/'tools/search_live_benchmark.py'), '--oracle', urls[0], '--candidate', urls[1], '--output', str(output), '--warmup', '1', '--samples', '5'], capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                text = (output/'report.json').read_text(); report = json.loads(text)
                self.assertEqual(report['status'], 'COMPLETED_RESTRICTED_CASE')
                self.assertEqual(report['timed_samples'], 20)
                self.assertNotIn('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', text)
        finally:
            for server in servers: server.shutdown(); server.server_close()
    def test_non_transport_header_difference_blocks_before_sampling(self):
        servers = [ThreadingHTTPServer(('127.0.0.1', 0), Handler) for _ in range(2)]
        for server in servers:
            server.line = '...'; threading.Thread(target=server.serve_forever, daemon=True).start()
        servers[0].extra_header = True
        try:
            with tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp)/'output'
                urls = ['http://127.0.0.1:%d/source/api/v1/search?path=a&start=0&maxresults=10' % s.server_port for s in servers]
                result = subprocess.run([sys.executable, str(ROOT/'tools/search_live_benchmark.py'), '--oracle', urls[0], '--candidate', urls[1], '--output', str(output), '--warmup', '1', '--samples', '5'], capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                report = json.loads((output/'report.json').read_text())
                self.assertEqual(report['status'], 'BLOCKED_HEADER_DIFFERENCE')
                self.assertEqual(report['timed_samples'], 0)
        finally:
            for server in servers: server.shutdown(); server.server_close()
    def test_session_cookie_difference_blocks_before_sampling(self):
        servers = [ThreadingHTTPServer(('127.0.0.1', 0), Handler) for _ in range(2)]
        for server in servers:
            server.line = '...'; threading.Thread(target=server.serve_forever, daemon=True).start()
        servers[0].cookie = True
        try:
            with tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp)/'output'
                urls = ['http://127.0.0.1:%d/source/api/v1/search?path=a&start=0&maxresults=10' % s.server_port for s in servers]
                result = subprocess.run([sys.executable, str(ROOT/'tools/search_live_benchmark.py'), '--oracle', urls[0], '--candidate', urls[1], '--output', str(output), '--warmup', '1', '--samples', '5'], capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                report_text = (output/'report.json').read_text(); report = json.loads(report_text)
                self.assertEqual(report['status'], 'BLOCKED_SESSION_BEHAVIOR')
                self.assertEqual(report['timed_samples'], 0)
                self.assertNotIn('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', report_text)
        finally:
            for server in servers: server.shutdown(); server.server_close()
    def test_mismatch_stops_before_sampling_and_keeps_both_raw_responses(self):
        servers = [ThreadingHTTPServer(('127.0.0.1', 0), Handler) for _ in range(2)]
        servers[0].line = '...'; servers[1].line = 'different'
        for server in servers: threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            with tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp)/'output'
                urls = ['http://127.0.0.1:%d/source/api/v1/search?path=a&start=0&maxresults=10' % s.server_port for s in servers]
                result = subprocess.run([sys.executable, str(ROOT/'tools/search_live_benchmark.py'), '--oracle', urls[0], '--candidate', urls[1], '--output', str(output), '--warmup', '1', '--samples', '5'], capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                report = json.loads((output/'report.json').read_text())
                self.assertEqual(report['status'], 'BLOCKED_OUTPUT_DIFFERENCE')
                self.assertEqual(report['timed_samples'], 0)
                self.assertNotEqual((output/'preflight-oracle.body').read_bytes(), (output/'preflight-candidate.body').read_bytes())
        finally:
            for server in servers: server.shutdown(); server.server_close()
if __name__ == '__main__': unittest.main()
