"""Loopback-only Tomcat configuration and non-redirecting HTTP clients."""
import urllib.request
import xml.etree.ElementTree as ET


def configure_server(path, port):
    tree = ET.parse(path)
    server = tree.getroot()
    connectors = server.findall('./Service/Connector')
    if len(connectors) != 1:
        raise ValueError('expected exactly one Tomcat connector for loopback-only oracle')
    if not 1 <= port <= 65535:
        raise ValueError('invalid connector port')
    server.set('port', '-1')
    connectors[0].set('address', '127.0.0.1')
    connectors[0].set('port', str(port))
    tree.write(path, encoding='utf-8', xml_declaration=True)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def opener(*handlers):
    return urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect(), *handlers)
