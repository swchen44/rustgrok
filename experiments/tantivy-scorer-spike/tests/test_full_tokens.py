"""Compare the public diagnostic token stream with pinned original output."""
from pathlib import Path
import subprocess
import unittest

ROOT=Path(__file__).resolve().parents[1]
REPO=ROOT.parents[1]
class FullTokens(unittest.TestCase):
    def test_ascii_token_positions_and_long_identifier_match_oracle(self):
        cases={'identifiers':'def main(): if __name__ == "__main__": main()', 'punctuation':'one_two 123 0x1F 12.34 @+$%&-={}[];<>/#?!~^','long_identifier':'a'*40000+' end'}
        for name,value in cases.items():
            expected=(REPO/'tests/evidence/full-token-diagnostic-2026-11-01'/f'{name}.oracle.txt').read_bytes()
            actual=subprocess.check_output(['cargo','run','--release','--locked','--offline','--quiet','--manifest-path',str(ROOT/'Cargo.toml'),'--example','full_token_probe','--',value])
            self.assertEqual(actual,expected,name)
if __name__=='__main__': unittest.main()
