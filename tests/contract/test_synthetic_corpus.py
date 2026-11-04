"""Synthetic corpus CLI: reproducibility, safe bounds and fail-closed verification."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
TOOL=ROOT/'tools/build_synthetic_corpus.py'


class SyntheticCorpus(unittest.TestCase):
    def test_new_sampled_reserve_is_4_5_gib_and_cannot_be_lower(self):
        sys.path.insert(0,str(ROOT/'tools'))
        import build_synthetic_corpus as synthetic
        required=9*1024**3//2
        self.assertEqual(synthetic.RESERVE,required)
        with patch.object(synthetic.shutil,'disk_usage') as usage:
            usage.return_value=type('Usage',(),{'free':required+1024})()
            synthetic.check_space(Path('/tmp'),required,0)
            with self.assertRaises(ValueError):synthetic.check_space(Path('/tmp'),required-1,0)

    def run_tool(self,*args):
        return subprocess.run([sys.executable,str(TOOL),*map(str,args)],capture_output=True,text=True)

    def test_reproducible_exact_size_and_mutation_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            a=Path(tmp)/'a';b=Path(tmp)/'b'
            for out in (a,b):
                run=self.run_tool('create','--output',out,'--projects',2,'--files-per-project',3,'--bytes-per-file',1024)
                self.assertEqual(run.returncode,0,run.stderr)
                self.assertEqual(self.run_tool('verify','--corpus',out).returncode,0)
            self.assertEqual((a/'manifest.json').read_bytes(),(b/'manifest.json').read_bytes())
            manifest=json.loads((a/'manifest.json').read_text())
            self.assertEqual(manifest['source_bytes'],6144)
            file=a/'source'/manifest['files'][0]['path']
            file.write_bytes(file.read_bytes()[:-1]+b'X')
            self.assertNotEqual(self.run_tool('verify','--corpus',a).returncode,0)

    def test_rejects_existing_path_and_insufficient_space_before_write(self):
        with tempfile.TemporaryDirectory() as tmp:
            parent=Path(tmp); existing=parent/'existing';existing.mkdir();(existing/'user.txt').write_text('keep')
            result=self.run_tool('create','--output',existing,'--projects',1,'--files-per-project',1,'--bytes-per-file',1024)
            self.assertNotEqual(result.returncode,0)
            self.assertEqual((existing/'user.txt').read_text(),'keep')
            output=parent/'unwritten'
            result=self.run_tool('create','--output',output,'--projects',1,'--files-per-project',1,'--bytes-per-file',1024,'--min-free-bytes',10**18)
            self.assertNotEqual(result.returncode,0)
            self.assertFalse(output.exists())

    def test_unexpected_file_and_symlink_block_verification(self):
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp)/'data';self.assertEqual(self.run_tool('create','--output',out,'--projects',1,'--files-per-project',1,'--bytes-per-file',1024).returncode,0)
            (out/'source'/'extra').write_text('x')
            self.assertNotEqual(self.run_tool('verify','--corpus',out).returncode,0)
            (out/'source'/'extra').unlink()
            (out/'source'/'extra').symlink_to(out/'manifest.json')
            self.assertNotEqual(self.run_tool('verify','--corpus',out).returncode,0)

    def test_manifest_boolean_does_not_equal_integer_project_count(self):
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp)/'data';self.assertEqual(self.run_tool('create','--output',out,'--projects',1,'--files-per-project',1,'--bytes-per-file',1024).returncode,0)
            path=out/'manifest.json';manifest=json.loads(path.read_text());manifest['projects']=True;path.write_text(json.dumps(manifest))
            self.assertNotEqual(self.run_tool('verify','--corpus',out).returncode,0)

if __name__=='__main__': unittest.main()
