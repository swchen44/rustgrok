"""Path-only HTTP response subset; does not assert full OpenGrok parity."""
import json
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import unittest
import urllib.request
import urllib.error

ROOT = Path(__file__).resolve().parents[1]

class PathResponse(unittest.TestCase):
    def test_path_query_returns_complete_search_result_shape(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); source = base / 'source'; (source / 'alpha').mkdir(parents=True)
            (source / 'alpha' / 'novel_42.c').write_text('int demo(void) { return 1; }\n')
            (source / 'beta').mkdir(); (source / 'beta' / 'other.c').write_text('int demo(void) { return 2; }\n')
            index = base / 'index.json'
            subprocess.run(['cargo', 'build', '--offline', '--quiet', '--manifest-path', str(ROOT/'Cargo.toml')], check=True)
            binary = ROOT/'target/debug/rustgrok-search-spike'
            subprocess.run([str(binary), 'index', '--source', str(source), '--index', str(index), '--ctags', '/opt/homebrew/bin/ctags'], check=True)
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
            proc = subprocess.Popen([str(binary), 'serve', '--index', str(index), '--listen', f'127.0.0.1:{port}'], stderr=subprocess.PIPE)
            try:
                for _ in range(40):
                    try:
                        with urllib.request.urlopen(f'http://127.0.0.1:{port}/source/api/v1/search?path=novel_42.c&projects=alpha&start=0&maxresults=10', timeout=1) as response:
                            payload = json.load(response); break
                    except OSError:
                        if proc.poll() is not None: self.fail('HTTP process exited before readiness')
                        time.sleep(.05)
                else: self.fail('HTTP readiness timeout')
                self.assertIsInstance(payload.pop('time'), int)
                self.assertEqual(payload, {'resultCount': 1, 'results': {'/alpha/novel_42.c': [{'line':'...', 'lineNumber':'', 'tag':None}]}, 'startDocument':0, 'endDocument':0})
                with urllib.request.urlopen(f'http://127.0.0.1:{port}/source/api/v1/search?path=absent.zz&projects=alpha&start=0&maxresults=10') as response:
                    empty = json.load(response)
                self.assertEqual(empty['resultCount'], 0)
                self.assertEqual(empty['results'], {})
                self.assertEqual((empty['startDocument'], empty['endDocument']), (0, 0))
                with self.assertRaises(urllib.error.HTTPError) as overflow:
                    urllib.request.urlopen(f'http://127.0.0.1:{port}/source/api/v1/search?path=novel_42.c&projects=alpha&start=2147483647&maxresults=1')
                self.assertEqual(overflow.exception.code, 400)
            finally:
                proc.terminate(); proc.wait(timeout=5); proc.stderr.close()

if __name__ == '__main__': unittest.main()
