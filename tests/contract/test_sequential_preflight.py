"""Sequential preflight requires strict status, body, headers and session behavior."""
from pathlib import Path
import sys
import unittest

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))

class SequentialPreflight(unittest.TestCase):
    def sample(self):
        body={'time':0,'resultCount':1,'results':{'/p/a':[{'line':'...', 'lineNumber':'', 'tag':None}]},'startDocument':0,'endDocument':0}
        first={'status':200,'headers':[('content-type','application/json'),('set-cookie','JSESSIONID=<redacted>; Path=/source; HttpOnly')],'cookie_present':True,'cookie_supported':True,'body':body}
        follow={'status':200,'headers':[('content-type','application/json')],'cookie_present':False,'cookie_supported':True,'body':body}
        return {'first':first,'followup':follow}

    def test_matching_observations_pass_but_header_or_session_differences_block(self):
        from sequential_preflight import equivalent
        a=self.sample();b=self.sample();self.assertTrue(equivalent(a,b))
        b['first']['headers'][-1]=('set-cookie','JSESSIONID=<redacted>; Path=/other; HttpOnly')
        self.assertFalse(equivalent(a,b))
        b=self.sample();b['followup']['cookie_present']=True
        self.assertFalse(equivalent(a,b))
        a=self.sample();b=self.sample();a['first']['cookie_present']=False;b['first']['cookie_present']=False
        self.assertFalse(equivalent(a,b))

    def test_distinct_json_types_or_non_200_status_block(self):
        from sequential_preflight import equivalent
        a=self.sample();b=self.sample();a['first']['body']['extra']=True;b['first']['body']['extra']=1
        self.assertFalse(equivalent(a,b))
        b=self.sample();b['first']['status']=302
        self.assertFalse(equivalent(self.sample(),b))

if __name__=='__main__':unittest.main()
