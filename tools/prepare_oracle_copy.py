#!/usr/bin/env python3
"""Make an isolated index and config for the restricted HTTP experiment."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import xml.etree.ElementTree as ET


def tree_hash(path):
    result = hashlib.sha256()
    for file in sorted(p for p in path.rglob('*') if p.is_file()):
        result.update(str(file.relative_to(path)).encode()); result.update(b'\0')
        result.update(hashlib.sha256(file.read_bytes()).digest())
    return result.hexdigest()


def prepare(capture, output):
    capture, output = capture.resolve(), output.resolve()
    if output == capture or capture in output.parents: raise ValueError('output must be outside capture')
    expected = json.loads((capture/'result.json').read_text())['configuration_sha256']
    if hashlib.sha256((capture/'configuration.xml').read_bytes()).hexdigest() != expected:
        raise ValueError('capture configuration does not match original receipt')
    before=tree_hash(capture)
    output.mkdir(parents=True,exist_ok=False)
    shutil.copytree(capture/'index',output/'index')
    tree=ET.parse(capture/'configuration.xml')
    data=tree.find('.//void[@property="dataRoot"]/string')
    sugg=tree.find('.//void[@property="suggesterConfig"]')
    if data is None or sugg is None: raise ValueError('missing original configuration fields')
    data.text=str(output/'index')
    for node in sugg.findall('./void[@property="allowMostPopular"]'): sugg.remove(node)
    ET.SubElement(ET.SubElement(sugg,'void',{'property':'allowMostPopular'}),'boolean').text='false'
    tree.write(output/'configuration.xml',encoding='utf-8',xml_declaration=True)
    after=tree_hash(capture)
    if before != after: raise ValueError('capture changed while preparing copy')
    return {'capture_tree_sha256_before':before,'capture_tree_sha256_after_prepare':after}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--capture',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    print(json.dumps(prepare(a.capture,a.output),indent=2))
if __name__=='__main__': main()
