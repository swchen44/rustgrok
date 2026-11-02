"""The planning package accepts intact evidence ZIPs and rejects corrupt ones."""
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))

class PlanningBinaryEvidence(unittest.TestCase):
    def test_zip_is_checked_without_decoding_as_utf8(self):
        from planning_files import read_package_text
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'evidence.zip'
            with zipfile.ZipFile(path, 'w') as archive:
                archive.writestr('sample.body', b'\xff\x00')
            self.assertIsNone(read_package_text(path))
            data = bytearray(path.read_bytes())
            offset = data.index(b'\xff\x00')
            data[offset] = 0xfe
            path.write_bytes(data)
            with self.assertRaises((ValueError, zipfile.BadZipFile)):
                read_package_text(path)
            path.write_bytes(b'not a ZIP')
            with self.assertRaises((ValueError, zipfile.BadZipFile)):
                read_package_text(path)

    def test_non_zip_invalid_text_fails_closed(self):
        from planning_files import read_package_text
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'source.txt'
            path.write_bytes(b'\xff')
            with self.assertRaises(UnicodeDecodeError):
                read_package_text(path)

if __name__ == '__main__':
    unittest.main()
