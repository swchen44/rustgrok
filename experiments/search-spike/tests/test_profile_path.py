"""Experimental profile preserves the path-search CLI result and reports its stages."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

SPIKE=Path(__file__).resolve().parents[1]

class PathProfile(unittest.TestCase):
    def test_profile_records_read_parse_scan_and_matches_baseline_result(self):
        with tempfile.TemporaryDirectory() as tmp:
            index=Path(tmp)/'index.json'
            docs=[{'path':'/alpha/main.c','project':'alpha','content':'source','definitions':[]},
                  {'path':'/alpha/other.c','project':'alpha','content':'other','definitions':[]},
                  {'path':'/beta/main.c','project':'beta','content':'third','definitions':[]}]
            index.write_text(json.dumps(docs))
            common=['--index',str(index),'--query','main.c','--project','alpha']
            args=['cargo','run','--release','--offline','--locked','--quiet','--manifest-path',str(SPIKE/'Cargo.toml')]
            original=json.loads(subprocess.check_output(args+['--','search','--field','path']+common,text=True))
            profile=json.loads(subprocess.check_output(args+['--example','profile_path','--']+common+['--samples','3'],text=True))
            self.assertEqual(profile['result'],original)
            self.assertEqual(profile['index_bytes'],index.stat().st_size)
            self.assertEqual((profile['documents'],profile['selected_documents']),(3,2))
            self.assertEqual(len(profile['samples']),3)
            for sample in profile['samples']:
                self.assertTrue(all(isinstance(sample[k],int) and sample[k]>=0 for k in ('read_ns','parse_ns','scan_total_ns','tokenize_ns','serialize_ns')))
                self.assertGreaterEqual(sample['scan_total_ns'],sample['tokenize_ns'])

if __name__=='__main__':unittest.main()
