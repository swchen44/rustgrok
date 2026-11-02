"""Verify retained original e04 planning-build receipt and encoded archive bytes."""
import base64
import hashlib
import io
import json
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET
import zipfile

ROOT=Path(__file__).resolve().parents[2]
EVIDENCE=ROOT/'tests/evidence/synthetic-20m-sequential-2026-11-01/planning-e04'


class PlanningE04Receipt(unittest.TestCase):
    def test_guard_stdout_and_exact_archive(self):
        guard=json.loads((EVIDENCE/'guard-receipt.json').read_text());stdout=json.loads((EVIDENCE/'planning.stdout.txt').read_text())
        proof=json.loads((EVIDENCE/'archive-verification.json').read_text())
        self.assertEqual(proof['archive_sha256'],'51870a6cb16de16e022b050fce098ce2b60e3013669438a7e9626f334f9b8f87')
        self.assertEqual(proof['manifest_matches_git_commit'],'e04d23b4e9f930653c57e533db8edf9c7c26d2b4')
        for name,entry in proof['raw_streams'].items():
            data=(EVIDENCE/name).read_bytes()
            self.assertEqual((len(data),hashlib.sha256(data).hexdigest()),(entry['bytes'],entry['sha256']))
        self.assertEqual((guard['status'],guard['exit_code'],guard['group_stopped']),('COMPLETED',0,True))
        self.assertFalse(guard['forced_cleanup']);self.assertFalse(stdout['product_tests_executed'])
        self.assertEqual((EVIDENCE/'planning.stderr.txt').read_bytes(),b'')
        raw=base64.b64decode((EVIDENCE/'package-e04.zip.base64').read_bytes(),validate=True)
        self.assertEqual(len(raw),stdout['zip_bytes']);self.assertEqual(hashlib.sha256(raw).hexdigest(),proof['archive_sha256'])
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            prefix='opengrok-host-planning/';manifest_raw=archive.read(prefix+'planning-manifest.json');manifest=json.loads(manifest_raw)
            self.assertEqual(hashlib.sha256(manifest_raw).hexdigest(),proof['manifest_sha256'])
            self.assertEqual(len(manifest['files']),proof['manifest_entries'])
            self.assertEqual(len(archive.namelist()),stdout['zip_files'])
            self.assertEqual(set(archive.namelist()),{prefix+x['path'] for x in manifest['files']}|{prefix+'planning-manifest.json'})
            for item in manifest['files']:
                with archive.open(prefix+item['path']) as stream:
                    digest=hashlib.sha256();size=0
                    for chunk in iter(lambda:stream.read(1024*1024),b''):digest.update(chunk);size+=len(chunk)
                self.assertEqual((size,digest.hexdigest()),(item['bytes'],item['sha256']),item['path'])

    def test_historical_ignored_logs_remain_in_zip_but_not_current_manifest(self):
        audit=json.loads((EVIDENCE/'archive-untracked-audit.json').read_text())
        entries=audit['archive_entries_not_tracked_by_e04'];self.assertEqual(len(entries),18)
        current={x['path'] for x in json.loads((ROOT/'planning-manifest.json').read_text())['files']}
        self.assertTrue(all(x['path'] not in current for x in entries))
        raw=base64.b64decode((EVIDENCE/'package-e04.zip.base64').read_bytes(),validate=True)
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            for item in entries:
                data=archive.read('opengrok-host-planning/'+item['path'])
                self.assertEqual((len(data),hashlib.sha256(data).hexdigest()),(item['bytes'],item['sha256']))
            receipt=(EVIDENCE/'original-public-capture-result.json').read_bytes()
            receipt_sha='0558697556728caa51894e8e7199fec11088c2ffe1afd35f15a86e61a82dbc83'
            self.assertEqual(hashlib.sha256(receipt).hexdigest(),receipt_sha)
            public_sha='88f491850b062ade1fbe157280f78b64b33774820c576dd83337cb11c7ab7c73'
            self.assertEqual(json.loads(receipt)['corpus_manifest_sha256'],public_sha)
            for name in ('project-order-original-2026-11-01','project-order-boundaries-2026-11-01'):
                result=json.loads(archive.read(f'opengrok-host-planning/tests/evidence/{name}/result.json'))
                self.assertEqual(result['capture_receipt_sha256'],receipt_sha)
            for name in ('live-path-benchmark-2026-11-01','live-path-benchmark-matched-2026-11-01',
                         'live-path-benchmark-tokenized-2026-11-01','live-path-cross-project-blocked-2026-11-01',
                         'live-path-no-hit-strict-2026-11-01','live-path-no-hit-verified-2026-11-01',
                         'live-path-single-hit-strict-2026-11-01','live-path-single-hit-verified-2026-11-01'):
                base=f'opengrok-host-planning/tests/evidence/{name}/'
                self.assertEqual(json.loads(archive.read(base+'metadata.json'))['public_corpus_manifest_sha256'],public_sha)
                node=ET.fromstring(archive.read(base+'configuration.xml')).find('.//void[@property="sourceRoot"]/string')
                self.assertTrue(node.text.endswith('/rustgrok-issue7-public-corpus/source'))

if __name__=='__main__':unittest.main()
