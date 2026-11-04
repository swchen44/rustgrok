"""The experimental runner must leave the original captured configuration intact."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
class OracleCopy(unittest.TestCase):
    def test_copy_and_modify_leaves_original_capture_byte_identical(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)/'capture'; (base/'index').mkdir(parents=True)
            (base/'configuration.xml').write_text('<java><void property="dataRoot"><string>old-index</string></void><void property="suggesterConfig"/></java>')
            (base/'index'/'data').write_bytes(b'index')
            digest = hashlib.sha256((base/'configuration.xml').read_bytes()).hexdigest()
            (base/'result.json').write_text(json.dumps({'configuration_sha256':digest}))
            target=Path(temp)/'copy'
            result=subprocess.run([sys.executable,str(ROOT/'tools/prepare_oracle_copy.py'),'--capture',str(base),'--output',str(target)],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual(hashlib.sha256((base/'configuration.xml').read_bytes()).hexdigest(),digest)
            self.assertEqual((base/'index'/'data').read_bytes(),b'index')
            self.assertIn('allowMostPopular', (target/'configuration.xml').read_text())
            self.assertIn(str(target/'index'), (target/'configuration.xml').read_text())
    def test_live_runner_rejects_output_inside_capture_before_creating_it(self):
        with tempfile.TemporaryDirectory() as temp:
            base=Path(temp)/'capture';base.mkdir();output=base/'new-output'
            result=subprocess.run([sys.executable,str(ROOT/'tools/run_live_path_experiment.py'),'--oracle-capture',str(base),'--corpus',temp,'--output',str(output)],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('output must be outside capture', result.stderr)
            self.assertFalse(output.exists())
            self.assertEqual(list(base.iterdir()),[])
    def test_rejects_capture_whose_configuration_hash_was_changed(self):
        with tempfile.TemporaryDirectory() as temp:
            base=Path(temp)/'capture';base.mkdir();(base/'configuration.xml').write_text('changed');(base/'result.json').write_text(json.dumps({'configuration_sha256':'0'*64}))
            result=subprocess.run([sys.executable,str(ROOT/'tools/prepare_oracle_copy.py'),'--capture',str(base),'--output',str(Path(temp)/'copy')],capture_output=True)
            self.assertNotEqual(result.returncode,0)
if __name__ == '__main__': unittest.main()
