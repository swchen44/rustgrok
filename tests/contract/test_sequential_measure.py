"""Sequential timing is eligible only after revalidating saved preflight bytes."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))

class SequentialMeasure(unittest.TestCase):
    def test_tampered_preflight_body_blocks_timing(self):
        from sequential_http_measure import load_eligibility
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);case='single-hit';body={'time':0,'resultCount':0,'results':{},'startDocument':0,'endDocument':0};raw=json.dumps(body).encode()
            first={'status':200,'headers':[['content-type','application/json'],['set-cookie','JSESSIONID=<redacted>; Path=/source; HttpOnly']],'cookie_present':True,'cookie_supported':True,'body':body}
            follow={'status':200,'headers':[['content-type','application/json']],'cookie_present':False,'cookie_supported':True,'body':body}
            obs={'first':first,'followup':follow};hashes={part:hashlib.sha256(raw).hexdigest() for part in ('first','followup')}
            for role in ('oracle','candidate'):
                for part in ('first','followup'):(root/f'preflight-{case}-{role}-{part}.body').write_bytes(raw)
            metadata={case:{'status':'MATCHED_PREFLIGHT_NOT_TIMED','query':'/source/api/v1/search?path=x','oracle':obs,'candidate':obs,'oracle_body_sha256':hashes,'candidate_body_sha256':hashes}}
            (root/'sequential-preflight.json').write_text(json.dumps(metadata))
            self.assertEqual(load_eligibility(root,case)['query'],metadata[case]['query'])
            (root/f'preflight-{case}-candidate-first.body').write_bytes(raw+b' ')
            with self.assertRaises(ValueError):load_eligibility(root,case)

if __name__=='__main__':unittest.main()
