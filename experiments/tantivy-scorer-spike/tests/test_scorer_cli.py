"""Public interface checks for the Tantivy G1 scorer candidate."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class ScorerProbe(unittest.TestCase):
    def test_selected_project_scores_repeated_term_ahead(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'documents.json'
            path.write_text(json.dumps([
                {'path': '/alpha/a.c', 'project': 'alpha', 'content': 'main x padding padding', 'definitions': []},
                {'path': '/alpha/b.c', 'project': 'alpha', 'content': 'main main padding padding', 'definitions': []},
                {'path': '/beta/c.c', 'project': 'beta', 'content': 'main main main main', 'definitions': []},
            ]))
            cmd = ['cargo', 'run', '--release', '--locked', '--offline', '--quiet', '--manifest-path', str(ROOT / 'Cargo.toml'), '--', '--documents', str(path), '--field', 'full', '--query', 'main', '--project', 'alpha']
            data = json.loads(subprocess.check_output(cmd, text=True))
            self.assertEqual([h['path'] for h in data['hits']], ['/alpha/b.c', '/alpha/a.c'])
            self.assertGreater(data['hits'][0]['score'], data['hits'][1]['score'])
            self.assertEqual(data['indexed_documents'], 2)

if __name__ == '__main__':
    unittest.main()
