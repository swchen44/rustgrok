"""The planning package accepts intact evidence ZIPs and rejects corrupt ones."""
from pathlib import Path
import sys
import tempfile
import unittest
import warnings
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

    def test_duplicate_names_cannot_hide_a_corrupted_first_member(self):
        from planning_files import read_package_text
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'duplicate.zip'
            with warnings.catch_warnings():
                warnings.simplefilter('ignore', UserWarning)
                with zipfile.ZipFile(path, 'w', compression=zipfile.ZIP_STORED) as archive:
                    archive.writestr('same.txt', b'first-unique-content')
                    archive.writestr('same.txt', b'last-good-content')
            intact = path.read_bytes()
            data = bytearray(intact)
            offset = data.index(b'first-unique-content')
            data[offset] ^= 1
            path.write_bytes(data)
            # Demonstrate why testzip() alone was insufficient on this input.
            with zipfile.ZipFile(path) as archive:
                self.assertIsNone(archive.testzip())
                with self.assertRaises(zipfile.BadZipFile):
                    archive.read(archive.infolist()[0])
            with self.assertRaises((ValueError, zipfile.BadZipFile)):
                read_package_text(path)
            # An intact archive with ambiguous names is invalid too.
            path.write_bytes(intact)
            with self.assertRaises(ValueError):
                read_package_text(path)

    def test_all_unique_members_are_streamed_through_their_final_chunk(self):
        from planning_files import read_package_text
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'unique.zip'
            with zipfile.ZipFile(path, 'w', compression=zipfile.ZIP_STORED) as archive:
                archive.writestr('first.txt', b'first')
                archive.writestr('large.txt', b'x' * 131072 + b'end-marker')
            self.assertIsNone(read_package_text(path))
            data = bytearray(path.read_bytes())
            data[data.index(b'end-marker')] ^= 1
            path.write_bytes(data)
            with self.assertRaises(zipfile.BadZipFile):
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
