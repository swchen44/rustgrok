"""A syntactically valid but incomplete JSON index is currently accepted."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

BINARY = Path(__file__).resolve().parents[1] / 'target/release/rustgrok-search-spike'


class PartialIndexSemantics(unittest.TestCase):
    def test_valid_truncated_document_array_is_accepted_as_complete_index(self):
        documents = [
            {'path': '/alpha/first.c', 'project': 'alpha', 'content': 'first', 'definitions': []},
            {'path': '/alpha/second.c', 'project': 'alpha', 'content': 'second', 'definitions': []},
        ]
        with tempfile.TemporaryDirectory() as tmp:
            index = Path(tmp) / 'index.json'
            command = [str(BINARY), 'search', '--index', str(index), '--field', 'path',
                       '--query', 'c', '--project', 'alpha']
            def search_with(input_docs):
                index.write_text(json.dumps(input_docs))
                return json.loads(subprocess.check_output(command, text=True))
            complete = search_with(documents)
            partial = search_with(documents[:1])
        self.assertEqual([hit['path'] for hit in complete['results']],
                         ['/alpha/first.c', '/alpha/second.c'])
        self.assertEqual([hit['path'] for hit in partial['results']], ['/alpha/first.c'])

if __name__ == '__main__':
    unittest.main()
