"""The optional index cache must observe complete file changes and failures."""
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import unittest
import urllib.error
import urllib.request

BINARY = Path(__file__).resolve().parents[1] / 'target/release/rustgrok-search-spike'


def docs(name):
    return json.dumps([{'path': '/alpha/' + name + '.c', 'project': 'alpha',
                        'content': 'source', 'definitions': []}], separators=(',', ':')).encode()


class CacheContract(unittest.TestCase):
    def test_invalid_cache_mode_fails_closed(self):
        result = subprocess.run([str(BINARY), 'serve', '--index', '/missing', '--listen',
                                 '127.0.0.1:0', '--cache-mode', 'invalid'],
                                capture_output=True, timeout=2)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'cache', result.stderr)

    def test_update_failure_and_recovery_match_uncached_behavior(self):
        observed = []
        for mode in ('off', 'exact'):
            with tempfile.TemporaryDirectory() as tmp:
                index = Path(tmp) / 'index.json'
                index.write_bytes(docs('first'))
                with socket.socket() as sock:
                    sock.bind(('127.0.0.1', 0))
                    port = sock.getsockname()[1]
                server = subprocess.Popen([str(BINARY), 'serve', '--index', str(index), '--listen',
                                           f'127.0.0.1:{port}', '--cache-mode', mode],
                                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                try:
                    time.sleep(.1)
                    self.assertIsNone(server.poll(), 'server exited before ready')
                    url = f'http://127.0.0.1:{port}/probe/search?field=path&query=c&project=alpha'
                    def request():
                        try:
                            with urllib.request.urlopen(url, timeout=5) as response:
                                return response.status, json.load(response)
                        except urllib.error.HTTPError as error:
                            return error.code, json.load(error)
                    results = [request(), request()]
                    replacement = Path(tmp) / 'new'
                    replacement.write_bytes(docs('other'))
                    os.replace(replacement, index)
                    results.append(request())
                    times = index.stat()
                    index.write_bytes(docs('third'))  # same size and restored mtime
                    os.utime(index, ns=(times.st_atime_ns, times.st_mtime_ns))
                    results.append(request())
                    index.write_bytes(b'bad JSON')
                    results.append(request())
                    index.unlink()
                    results.append(request())
                    index.write_bytes(docs('final'))
                    results.append(request())
                    observed.append(results)
                finally:
                    server.terminate()
                    server.wait(timeout=5)
        for results in observed:
            self.assertEqual([status for status, _ in results], [200, 200, 200, 200, 400, 400, 200])
            paths = [results[i][1]['results'][0]['path'] for i in (0, 1, 2, 3, 6)]
            self.assertEqual(paths, ['/alpha/first.c', '/alpha/first.c', '/alpha/other.c',
                                     '/alpha/third.c', '/alpha/final.c'])
        self.assertEqual(observed[0], observed[1])

if __name__ == '__main__':
    unittest.main()
