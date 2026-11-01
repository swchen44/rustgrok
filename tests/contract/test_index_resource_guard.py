"""A monitored child is stopped when the disk safety reserve is breached."""
from pathlib import Path
import sys
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))

class ResourceGuard(unittest.TestCase):
    def test_low_space_stops_child_process_group(self):
        from run_synthetic_index_probe import run_guarded
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)
            availability=iter([10_000,0])
            result=run_guarded([sys.executable,'-c','import time;time.sleep(10)'],path,'test',dict(),reserve=100,
                               free_space=lambda:next(availability,0),interval=.05,timeout=5)
            self.assertEqual(result['status'],'STOPPED_DISK_RESERVE')
            self.assertNotEqual(result['exit_code'],0)
            self.assertTrue((path/'test.stderr.txt').exists())

if __name__=='__main__': unittest.main()
