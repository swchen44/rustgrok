"""Black-box contract for the public corpus generation command."""
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMMAND = ROOT / 'tools/build_public_corpus.py'


class PublicCorpusContract(unittest.TestCase):
    def test_rebuilds_identical_projects_repositories_and_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            manifests = []
            for name in ('first', 'second'):
                dest = Path(tmp) / name
                environment = os.environ.copy()
                if name == 'second':
                    environment.update(GIT_AUTHOR_NAME='Ambient Author', GIT_AUTHOR_EMAIL='ambient@example.invalid',
                                       GIT_COMMITTER_NAME='Ambient Committer', GIT_COMMITTER_EMAIL='ambient@example.invalid')
                result = subprocess.run(['python3', str(COMMAND), '--output', str(dest)], capture_output=True, text=True, env=environment)
                self.assertEqual(result.returncode, 0, result.stderr)
                manifest = json.loads((dest / 'manifest.json').read_text())
                self.assertEqual({x['project'] for x in manifest['repositories']}, {'alpha', 'beta'})
                self.assertEqual(len([x for x in manifest['repositories'] if x['project'] == 'alpha']), 3)
                self.assertIn('alpha/component-c/nested-js', {x['path'] for x in manifest['repositories']})
                self.assertEqual({x['language'] for x in manifest['repositories']}, {'C', 'Python', 'JavaScript'})
                for entry in manifest['repositories']:
                    repo = dest / 'source' / entry['path']
                    head = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
                    self.assertEqual(head, entry['revision'])
                    self.assertEqual(subprocess.check_output(['git', '-C', str(repo), 'status', '--porcelain'], text=True), '')
                manifests.append(manifest)
            self.assertEqual(manifests[0], manifests[1])
            c_files = [x for x in manifests[0]['repositories'] if x['language'] == 'C']
            self.assertEqual(len(c_files), 2)
            self.assertEqual(c_files[0]['source_sha256'], c_files[1]['source_sha256'])

    def test_refuses_to_overwrite_existing_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            dest = Path(tmp) / 'existing'; dest.mkdir()
            sentinel = dest / 'keep'; sentinel.write_text('preserve')
            result = subprocess.run(['python3', str(COMMAND), '--output', str(dest)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('already exists', result.stderr)
            self.assertEqual(sentinel.read_text(), 'preserve')


if __name__ == '__main__':
    unittest.main()
