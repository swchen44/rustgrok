"""Planning package inputs are explicitly staged, not ambient ignored files."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))

class PlanningFiles(unittest.TestCase):
    def test_only_index_paths_are_selected_and_forced_log_is_explicit(self):
        from planning_files import package_files
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);subprocess.run(['git','init','-q',str(root)],check=True)
            (root/'.gitignore').write_text('*.log\n.env\n');(root/'visible.txt').write_text('public');(root/'ambient.txt').write_text('untracked')
            (root/'.env').write_text('private');(root/'ignored.log').write_text('private');(root/'approved.log').write_text('reviewed')
            subprocess.run(['git','-C',str(root),'add','.gitignore','visible.txt'],check=True)
            subprocess.run(['git','-C',str(root),'add','-f','approved.log'],check=True)
            with patch.dict(os.environ,{'GIT_DIR':'/nonexistent/ambient-git'}):paths={p.name for p in package_files(root)}
            self.assertEqual(paths,{'.gitignore','visible.txt','approved.log'})

    def test_unstaged_tracked_changes_block_package_construction(self):
        from planning_files import require_staged_worktree
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);subprocess.run(['git','init','-q',str(root)],check=True)
            (root/'f').write_text('staged');subprocess.run(['git','-C',str(root),'add','f'],check=True)
            require_staged_worktree(root)
            (root/'f').write_text('different')
            with self.assertRaises(ValueError):require_staged_worktree(root)

    def test_hidden_index_flags_cannot_hide_modified_worktree_bytes(self):
        from planning_files import package_files, require_staged_worktree
        for flag in ('--assume-unchanged','--skip-worktree'):
            with self.subTest(flag=flag),tempfile.TemporaryDirectory() as tmp:
                root=Path(tmp);subprocess.run(['git','init','-q',str(root)],check=True)
                (root/'f').write_text('reviewed staged content');subprocess.run(['git','-C',str(root),'add','f'],check=True)
                subprocess.run(['git','-C',str(root),'update-index',flag,'f'],check=True)
                with self.assertRaises(ValueError):require_staged_worktree(root)
                with self.assertRaises(ValueError):package_files(root)
                (root/'f').write_text('unreviewed worktree content')
                # These flags demonstrably make the ordinary index/worktree diff lie.
                self.assertEqual(subprocess.run(['git','-C',str(root),'diff','--quiet']).returncode,0)
                with self.assertRaises(ValueError):require_staged_worktree(root)
                with self.assertRaises(ValueError):package_files(root)

if __name__=='__main__':unittest.main()
