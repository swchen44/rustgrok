"""Compare a complete path term and a substring against the fixed oracle."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[3]
SPIKE=ROOT/'experiments/search-spike'
class PathTerms(unittest.TestCase):
    def test_public_corpus_term_and_substring_counts_match_oracle(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp=Path(tmp);corpus=tmp/'corpus';index=tmp/'index.json'
            subprocess.run(['python3',str(ROOT/'tools/build_public_corpus.py'),'--output',str(corpus)],check=True,stdout=subprocess.DEVNULL)
            subprocess.run(['cargo','build','--locked','--offline','--quiet','--manifest-path',str(SPIKE/'Cargo.toml')],check=True)
            binary=SPIKE/'target/debug/rustgrok-search-spike'
            subprocess.run([str(binary),'index','--source',str(corpus/'source'),'--index',str(index),'--ctags','/opt/homebrew/bin/ctags'],check=True,stdout=subprocess.DEVNULL)
            for term in ('main','mai'):
                raw=(ROOT/'tests/evidence/path-ranking-diagnostic-2026-11-01'/(term+'.oracle.txt')).read_text()
                expected=sum(line.startswith('doc=') for line in raw.splitlines())
                got=subprocess.check_output([str(binary),'search','--index',str(index),'--field','path','--query',term,'--project','alpha','--project','beta'])
                self.assertEqual(json.loads(got)['resultCount'],expected,term)
if __name__=='__main__': unittest.main()
