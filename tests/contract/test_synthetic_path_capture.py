"""Synthetic oracle path capture never overwrites an existing output."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]

class SyntheticPathCapture(unittest.TestCase):
    def test_benchmark_rejects_invalid_sample_count_before_output_creation(self):
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp);out=path/'output'
            result=subprocess.run([sys.executable,str(ROOT/'tools/verify_synthetic_paths.py'),'--corpus',str(path),'--probe',str(path),'--output',str(out),'--benchmark','--samples','3'],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('samples must be a multiple of five',result.stderr)
            self.assertFalse(out.exists())

    def test_existing_output_is_not_modified(self):
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp);out=path/'output';out.mkdir();(out/'user.txt').write_text('keep')
            result=subprocess.run([sys.executable,str(ROOT/'tools/verify_synthetic_paths.py'),'--corpus',str(path),'--probe',str(path),'--output',str(out)],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('output must not exist',result.stderr)
            self.assertEqual((out/'user.txt').read_text(),'keep')

if __name__=='__main__':unittest.main()
