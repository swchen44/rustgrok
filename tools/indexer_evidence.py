"""Preserve original indexer logs and check the public benchmark's file coverage."""
from collections import Counter
import hashlib
import json
import re


def record_indexer_logs(capture, output):
    streams = {}
    for name in ('stdout', 'stderr'):
        data = (capture / ('index.' + name + '.txt')).read_bytes()
        filename = 'original-indexer.' + name + '.txt'
        (output / filename).write_bytes(data)
        streams[name] = {'file': filename, 'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
        if name == 'stderr': stderr = data.decode('utf-8', errors='replace')
    patterns = (re.compile(r'\bERROR\s+addFile\(\)', re.I),
                re.compile(r'\b[1-9][0-9]*\s+failures?\b.*\b(?:while|during)\s+(?:parallel-)?indexing\b', re.I))
    indicators = [line for line in stderr.splitlines() if any(p.search(line) for p in patterns)]
    markers = [phrase for phrase in ('Done indexing data of all repositories', 'Indexer finished with success') if phrase in stderr]
    status = ('BLOCKED_KNOWN_FILE_FAILURE' if indicators else
              'NO_KNOWN_FILE_FAILURE_INDICATORS' if len(markers) == 2 else 'BLOCKED_MISSING_COMPLETION_MARKER')
    report = {'status':status, 'streams':streams, 'indicators':indicators, 'completion_markers':markers,
              'limitation':'Absence of these particular log messages alone does not prove all source files were indexed.'}
    (output/'original-indexer-log-check.json').write_text(json.dumps(report,indent=2)+'\n')
    return report


def verify_path_coverage(manifest, responses):
    expected = ['/' + r['path'].strip('/') + '/' + r['file'] for r in manifest['repositories']]
    observed = []
    valid = len(expected) == len(set(expected))
    for basename in sorted({r['file'] for r in manifest['repositories']}):
        body = responses.get(basename)
        if not isinstance(body, dict) or not isinstance(body.get('results'), dict):
            valid = False
            continue
        paths = list(body['results'])
        if type(body.get('resultCount')) is not int or body['resultCount'] != len(paths):
            valid = False
        observed.extend(paths)
    expected_counts, observed_counts = Counter(expected), Counter(observed)
    return {'complete': valid and observed_counts == expected_counts,
            'expected_paths': expected, 'observed_paths': observed,
            'limitation':'Path search covers this fixed corpus only; it does not prove full source contents or all query features.'}


def verify_candidate_coverage(expected, documents):
    if not isinstance(documents, list) or any(not isinstance(d, dict) for d in documents):
        return {'complete':False, 'reason':'invalid candidate index shape'}
    counts = Counter(d.get('path') for d in documents)
    complete = counts == Counter(expected.keys()) and all(d.get('content') == expected.get(d.get('path')) for d in documents)
    return {'complete':complete, 'expected_paths':list(expected), 'observed_paths':[d.get('path') for d in documents],
            'limitation':'Candidate source contents checked for this fixture only; it does not prove equal analyzers or all query features.'}
