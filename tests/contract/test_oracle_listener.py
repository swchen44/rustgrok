"""The original oracle must bind only the loopback interface."""
from pathlib import Path
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))

class OracleListener(unittest.TestCase):
    def test_server_configuration_disables_shutdown_and_binds_loopback(self):
        from oracle_http import configure_server
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'server.xml'
            path.write_text('<Server port="8005"><Service><Connector port="8080" /></Service></Server>')
            configure_server(path, 12001)
            root = ET.parse(path).getroot()
            self.assertEqual(root.get('port'), '-1')
            connector = root.find('./Service/Connector')
            self.assertEqual(connector.get('address'), '127.0.0.1')
            self.assertEqual(connector.get('port'), '12001')

    def test_multiple_connectors_are_rejected(self):
        from oracle_http import configure_server
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'server.xml'
            path.write_text('<Server port="8005"><Service><Connector/><Connector/></Service></Server>')
            with self.assertRaises(ValueError): configure_server(path, 12001)

if __name__ == '__main__': unittest.main()
