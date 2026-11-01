#!/usr/bin/env python3
"""Gate HTTP search timing on equal complete ordered OpenGrok-shaped responses."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import sys


def no_duplicate_keys(pairs):
    output = {}
    for key, value in pairs:
        if key in output:
            raise ValueError('duplicate JSON object key')
        output[key] = value
    return output


def load_response(path):
    raw = path.read_bytes()
    value = json.loads(raw, object_pairs_hook=no_duplicate_keys, parse_float=finite_float,
                       parse_constant=lambda value: (_ for _ in ()).throw(ValueError('non-finite JSON number')))
    return raw, value


def finite_float(value):
    result = float(value)
    if not math.isfinite(result):
        raise ValueError('non-finite JSON number')
    return result


def strict_equal(left, right):
    """JSON types must match recursively; integer and float parses stay distinct."""
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(strict_equal(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(strict_equal(a, b) for a, b in zip(left, right))
    return left == right


def valid_shape(obj):
    required = {'time', 'resultCount', 'results', 'startDocument', 'endDocument'}
    if not isinstance(obj, dict) or not required.issubset(obj):
        return False
    if any(not isinstance(obj[key], int) or isinstance(obj[key], bool) or obj[key] < 0
           for key in ('resultCount', 'startDocument', 'endDocument')):
        return False
    if not isinstance(obj['time'], (int, float)) or isinstance(obj['time'], bool):
        return False
    if not isinstance(obj['results'], dict):
        return False
    for path, hits in obj['results'].items():
        if not isinstance(path, str) or not isinstance(hits, list):
            return False
        for hit in hits:
            if not isinstance(hit, dict) or not {'line', 'lineNumber', 'tag'}.issubset(hit):
                return False
            if not isinstance(hit['line'], str) or not isinstance(hit['lineNumber'], str):
                return False
            if hit['tag'] is not None and not isinstance(hit['tag'], str):
                return False
    return True


def comparison(oracle, candidate):
    if not valid_shape(oracle) or not valid_shape(candidate):
        return False
    # Only this field is a non-deterministic measurement rather than search content.
    left = {k: v for k, v in oracle.items() if k not in {'time', 'results'}}
    right = {k: v for k, v in candidate.items() if k not in {'time', 'results'}}
    # Python dict equality ignores order. Explicitly preserve the result path sequence.
    return (strict_equal(left, right) and list(oracle['results']) == list(candidate['results'])
            and all(strict_equal(oracle['results'][key], candidate['results'][key]) for key in oracle['results']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--oracle', required=True, type=Path)
    parser.add_argument('--candidate', required=True, type=Path)
    parser.add_argument('--oracle-status', required=True, type=int)
    parser.add_argument('--candidate-status', required=True, type=int)
    args = parser.parse_args()
    report = {'schema_version': 1, 'normalization': ['top-level time value'],
              'oracle_status': args.oracle_status, 'candidate_status': args.candidate_status,
              'timing_performed': False, 'full_product_parity': 'NOT_ASSERTED',
              'other_preconditions': 'Request, corpus, headers, configuration, environment and warmup must also be verified before timing.'}
    try:
        left_raw, left = load_response(args.oracle)
        right_raw, right = load_response(args.candidate)
        report['oracle_sha256'] = hashlib.sha256(left_raw).hexdigest()
        report['candidate_sha256'] = hashlib.sha256(right_raw).hexdigest()
        if args.oracle_status != 200 or args.candidate_status != 200:
            report['status'] = 'BLOCKED_HTTP_STATUS'
        elif comparison(left, right):
            report['status'] = 'BODY_MATCH_PRECHECK_ONLY'
        else:
            report['status'] = 'BLOCKED_OUTPUT_DIFFERENCE'
    except (OSError, UnicodeDecodeError, ValueError) as e:
        report['status'] = 'BLOCKED_INVALID_RESPONSE'
        report['error'] = str(e)
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if report['status'] == 'BODY_MATCH_PRECHECK_ONLY' else 1

if __name__ == '__main__':
    sys.exit(main())
