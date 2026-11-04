"""The sequential path diagnostic must require proof that the original server stopped."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]

class SequentialPaths(unittest.TestCase):
    def test_missing_prior_stopped_process_proof_refuses_before_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            base=Path(tmp);prior=base/'prior';prior.mkdir();(prior/'coverage.json').write_text(json.dumps({'status':'MATCHED_ALL_SYNTHETIC_PATHS','process_groups_stopped':{'123':False}}))
            out=base/'out'
            cmd=[sys.executable,str(ROOT/'tools/compare_synthetic_paths_sequential.py'),'--corpus',str(base),'--probe',str(base),'--original',str(prior),'--output',str(out)]
            result=subprocess.run(cmd,capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('original process groups are not proven stopped',result.stderr)
            self.assertFalse(out.exists())

if __name__=='__main__':unittest.main()
