"""Behavioral contract for the experimental index and search CLI."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import threading
import socket
import time
import urllib.request
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class SearchCli(unittest.TestCase):
    def test_indexes_public_project_and_selects_fulltext_hits(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); source = base / 'source'
            (source / 'alpha').mkdir(parents=True)
            (source / 'alpha' / 'sample.c').write_text('int main(void) { return 0; }\n')
            (source / 'beta').mkdir()
            (source / 'beta' / 'other.c').write_text('int main(void) { return 1; }\n')
            index = base / 'index.json'
            command = ['cargo', 'run', '--offline', '--quiet', '--manifest-path', str(ROOT / 'Cargo.toml'), '--']
            subprocess.run(command + ['index', '--source', str(source), '--index', str(index), '--ctags', '/opt/homebrew/bin/ctags'], check=True)
            result = subprocess.check_output(command + ['search', '--index', str(index), '--field', 'full', '--query', 'main', '--project', 'alpha'], text=True)
            data = json.loads(result)
            self.assertEqual(data['resultCount'], 1)
            self.assertEqual([item['path'] for item in data['results']], ['/alpha/sample.c'])
            self.assertEqual(data['results'][0]['lineNumbers'], [1])

    def test_path_definition_and_reference_search(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); source = base / 'source'; (source / 'alpha').mkdir(parents=True)
            (source / 'alpha' / 'entry.c').write_text('int main(void) { return 0; }\nint f(void) { return main(); }\n')
            index = base / 'index.json'
            cmd = ['cargo', 'run', '--offline', '--quiet', '--manifest-path', str(ROOT / 'Cargo.toml'), '--']
            subprocess.run(cmd + ['index', '--source', str(source), '--index', str(index), '--ctags', '/opt/homebrew/bin/ctags'], check=True)
            def query(field, text):
                return json.loads(subprocess.check_output(cmd + ['search', '--index', str(index), '--field', field, '--query', text], text=True))
            self.assertEqual(query('path', 'entry.c')['results'][0]['path'], '/alpha/entry.c')
            self.assertEqual(query('def', 'main')['results'][0]['lineNumbers'], [1])
            self.assertEqual(query('symbol', 'main')['results'][0]['lineNumbers'], [1, 2])

    def test_http_entry_exposes_same_search(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); source = base / 'source'; (source / 'alpha').mkdir(parents=True)
            (source / 'alpha' / 'a.c').write_text('int main(void) { return 0; }\n')
            index = base / 'index.json'
            subprocess.run(['cargo', 'build', '--offline', '--quiet', '--manifest-path', str(ROOT / 'Cargo.toml')], check=True)
            binary = str(ROOT / 'target/debug/rustgrok-search-spike')
            subprocess.run([binary, 'index', '--source', str(source), '--index', str(index), '--ctags', '/opt/homebrew/bin/ctags'], check=True)
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
            proc = subprocess.Popen([binary, 'serve', '--index', str(index), '--listen', f'127.0.0.1:{port}'], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            try:
                for attempt in range(40):
                    try:
                        with urllib.request.urlopen(f'http://127.0.0.1:{port}/probe/search?field=full&query=main&project=alpha', timeout=1) as response:
                            data = json.load(response)
                        break
                    except OSError:
                        if proc.poll() is not None: self.fail('HTTP process exited before readiness')
                        time.sleep(0.05)
                else: self.fail('HTTP readiness timeout')
                self.assertEqual(data['resultCount'], 1)
                self.assertEqual(data['results'][0]['path'], '/alpha/a.c')
            finally:
                proc.terminate(); proc.wait(timeout=5); proc.stderr.close()

    def test_index_rejects_symlinked_destination_inside_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); source = base / 'source'; (source / 'alpha').mkdir(parents=True)
            (source / 'alpha' / 'a.c').write_text('int x;\n')
            (base / 'alias').symlink_to(source, target_is_directory=True)
            cmd = ['cargo', 'run', '--offline', '--quiet', '--manifest-path', str(ROOT / 'Cargo.toml'), '--']
            for destination in (base / 'alias' / 'index.json', source / 'alpha' / '..' / 'index.json'):
                p = subprocess.run(cmd + ['index', '--source', str(source), '--index', str(destination), '--ctags', '/opt/homebrew/bin/ctags'], capture_output=True)
                self.assertNotEqual(p.returncode, 0)
                self.assertFalse((source / 'index.json').exists())

    def test_differential_reports_order_difference_without_sorting(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); oracle = base / 'oracle'; oracle.mkdir()
            payload = {'resultCount': 2, 'results': {'/a': [{'lineNumber': '1'}], '/b': [{'lineNumber': '2'}]}}
            for case in ('full_alpha','path_alpha','definition_alpha','reference_beta','full_both','definition_beta','reference_alpha'):
                (oracle / f'response-{case}.json').write_text(json.dumps(payload))
            binary = base / 'candidate'; binary.write_text('#!/usr/bin/env python3\nimport json\nprint(json.dumps({"resultCount":2,"results":[{"path":"/b","lineNumbers":[2]},{"path":"/a","lineNumbers":[1]}]}))\n'); binary.chmod(0o755)
            output = base / 'out'
            proc = subprocess.run(['python3', str(ROOT / 'differential.py'), '--binary', str(binary), '--index', str(base / 'index'), '--oracle', str(oracle), '--output', str(output)], capture_output=True)
            self.assertEqual(proc.returncode, 1)
            report = json.loads((output / 'comparison.json').read_text())
            self.assertEqual(len(report['cases']), 7)
            self.assertFalse(report['cases'][0]['ordered_paths_match'])
            self.assertTrue(report['cases'][0]['line_numbers_by_path_match'])
            self.assertEqual(report['cases'][0]['candidate_ordered_paths'], ['/b','/a'])

    def test_differential_uses_loopback_http_and_preserves_order(self):
        seen = []
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                seen.append(self.path)
                body = json.dumps({'resultCount': 2, 'results': [{'path': '/b', 'lineNumbers': [2]}, {'path': '/a', 'lineNumbers': [1]}]}).encode()
                self.send_response(200); self.send_header('Content-Length', str(len(body))); self.end_headers(); self.wfile.write(body)
            def log_message(self, *args): pass
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever); thread.start()
        try:
            with tempfile.TemporaryDirectory() as tmp:
                base = Path(tmp); oracle = base / 'oracle'; oracle.mkdir()
                payload = {'resultCount':2, 'results':{'/a':[{'lineNumber':'1'}],'/b':[{'lineNumber':'2'}]}}
                for case in ('full_alpha','path_alpha','definition_alpha','reference_beta','full_both','definition_beta','reference_alpha'):
                    (oracle / f'response-{case}.json').write_text(json.dumps(payload))
                proc = subprocess.run(['python3', str(ROOT/'differential.py'), '--http-base', f'http://127.0.0.1:{server.server_port}', '--oracle', str(oracle), '--output', str(base/'out')], capture_output=True)
                self.assertEqual(proc.returncode, 1)
                report=json.loads((base/'out/comparison.json').read_text())
                self.assertEqual(len(seen), 7)
                self.assertTrue(all(r['candidate_http_status']==200 for r in report['cases']))
                self.assertFalse(report['cases'][0]['ordered_paths_match'])
        finally:
            server.shutdown(); thread.join(); server.server_close()

    def test_public_fixture_definition_and_reference_lines_for_priority_languages(self):
        # This checks hit membership and line numbers only. The separate differential
        # harness compares emitted path order without sorting or masking it.
        repo = ROOT.parents[1]
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); corpus = base / 'corpus'; index = base / 'index.json'
            subprocess.run(['python3', str(repo/'tools/build_public_corpus.py'), '--output', str(corpus)], check=True, capture_output=True)
            cmd=['cargo','run','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--']
            subprocess.run(cmd+['index','--source',str(corpus/'source'),'--index',str(index),'--ctags','/opt/homebrew/bin/ctags'],check=True)
            originals=repo/'tests/evidence/search-spike-2026-11-01/oracle-inputs'
            for case, field, project in [('definition_alpha','def','alpha'),('definition_beta','def','beta'),('reference_alpha','symbol','alpha'),('reference_beta','symbol','beta')]:
                original=json.loads((originals/f'response-{case}.json').read_text())
                candidate=json.loads(subprocess.check_output(cmd+['search','--index',str(index),'--field',field,'--query','main','--project',project],text=True))
                expected={path:[int(v['lineNumber']) for v in values if v['lineNumber']] for path,values in original['results'].items()}
                actual={item['path']:item['lineNumbers'] for item in candidate['results']}
                self.assertEqual(candidate['resultCount'],original['resultCount'],case)
                self.assertEqual(actual,expected,case)

    def test_unsupported_query_is_explicit_error(self):
        # Never silently reinterpret a Boolean expression as a simple term.
        cmd=['cargo','run','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--','search','--index','/unused','--field','full','--query','main OR other']
        result=subprocess.run(cmd,capture_output=True,text=True)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('only one identifier query is supported',result.stderr)

    def test_differential_rejects_non_loopback_before_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            base=Path(tmp); out=base/'out'
            proc=subprocess.run(['python3',str(ROOT/'differential.py'),'--http-base','http://192.0.2.1:1234','--oracle',str(base),'--output',str(out)],capture_output=True,text=True)
            self.assertNotEqual(proc.returncode,0)
            self.assertIn('loopback',proc.stderr)
            self.assertFalse(out.exists())

if __name__ == '__main__':
    unittest.main()
