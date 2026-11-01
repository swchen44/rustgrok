#!/usr/bin/env python3
"""Compare five original responses with this experiment, preserving path order."""
import argparse
import hashlib
import ipaddress
import urllib.parse
import urllib.request
import urllib.error
import json
from pathlib import Path
import subprocess
import sys

CASES = (
    ('full_alpha', 'full', 'main', ('alpha',)),
    ('path_alpha', 'path', 'main.c', ('alpha',)),
    ('definition_alpha', 'def', 'main', ('alpha',)),
    ('reference_beta', 'symbol', 'main', ('beta',)),
    ('full_both', 'full', 'main', ('alpha', 'beta')),
    ('definition_beta', 'def', 'main', ('beta',)),
    ('reference_alpha', 'symbol', 'main', ('alpha',)),
)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--binary', type=Path)
    modes.add_argument('--http-base')
    parser.add_argument('--index', type=Path)
    parser.add_argument('--oracle', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.binary and not args.index: parser.error('--index is required with --binary')
    if args.http_base:
        endpoint = urllib.parse.urlsplit(args.http_base)
        try:
            valid = endpoint.scheme == 'http' and endpoint.hostname is not None and ipaddress.ip_address(endpoint.hostname).is_loopback and not endpoint.path and not endpoint.query and not endpoint.fragment and endpoint.username is None
        except ValueError:
            valid = False
        if not valid: parser.error('--http-base must be an HTTP loopback IP and port without path')
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    if args.output.exists():
        parser.error('output already exists')
    args.output.mkdir(parents=True)
    cases = []
    for name, field, query, projects in CASES:
        original = (args.oracle / ('response-'+name+'.json')).read_bytes()
        orig = json.loads(original)
        if args.binary:
            cmd = [str(args.binary), 'search', '--index', str(args.index), '--field', field, '--query', query]
            for project in projects: cmd += ['--project', project]
            process = subprocess.run(cmd, capture_output=True)
            stdout, stderr, exit_code, http_status = process.stdout, process.stderr, process.returncode, None
        else:
            params = [('field', field), ('query', query)] + [('project', p) for p in projects]
            try:
                with opener.open(args.http_base + '/probe/search?' + urllib.parse.urlencode(params), timeout=10) as response:
                    stdout, stderr, http_status = response.read(), b'', response.status
            except urllib.error.HTTPError as error:
                stdout, stderr, http_status = error.read(), str(error).encode(), error.code
            except OSError as error:
                stdout, stderr, http_status = b'', str(error).encode(), None
            exit_code = 0 if http_status == 200 else 1
        (args.output / (name+'.oracle.json')).write_bytes(original)
        (args.output / (name+'.candidate.json')).write_bytes(stdout)
        (args.output / (name+'.candidate.stderr.txt')).write_bytes(stderr)
        case = {'name': name, 'candidate_exit_code': exit_code, 'candidate_http_status': http_status,
                'oracle_sha256': hashlib.sha256(original).hexdigest(),
                'candidate_sha256': hashlib.sha256(stdout).hexdigest()}
        if exit_code:
            case['comparison'] = 'candidate_error'
        else:
            candidate = json.loads(stdout)
            original_paths = list(orig['results'])
            candidate_paths = [item['path'] for item in candidate['results']]
            if len(set(candidate_paths)) != len(candidate_paths): raise ValueError('duplicate candidate path')
            def numbers(hits):
                # OpenGrok uses an empty lineNumber for path-only results.
                return [int(hit['lineNumber']) for hit in hits if hit['lineNumber'] != '']
            original_lines = {path: numbers(hits) for path, hits in orig['results'].items()}
            candidate_lines = {item['path']: item['lineNumbers'] for item in candidate['results']}
            case.update(oracle_count=orig['resultCount'], candidate_count=candidate['resultCount'],
                        count_match=orig['resultCount']==candidate['resultCount'],
                        oracle_ordered_paths=original_paths, candidate_ordered_paths=candidate_paths,
                        ordered_paths_match=original_paths==candidate_paths,
                        oracle_line_numbers_by_path=original_lines, candidate_line_numbers_by_path=candidate_lines,
                        line_numbers_by_path_match=original_lines==candidate_lines)
        cases.append(case)
    report = {'scope': 'limited G1 comparison; not product or full parity acceptance',
              'comparison_projection': 'resultCount, ordered paths and line numbers; empty original lineNumber means no source line; original time and snippet/tag are outside this projection and raw responses remain unchanged',
              'sort_or_reorder_applied': False, 'cases': cases}
    (args.output / 'comparison.json').write_text(json.dumps(report, indent=2)+'\n')
    return 0 if all(c.get('count_match') and c.get('ordered_paths_match') and c.get('line_numbers_by_path_match') for c in cases) else 1

if __name__ == '__main__':
    sys.exit(main())
