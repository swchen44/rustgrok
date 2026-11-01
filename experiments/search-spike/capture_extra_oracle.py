#!/usr/bin/env python3
"""Capture two additional original-OpenGrok cases using the verified #5 runner."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import capture_opengrok_baseline as oracle  # noqa: E402

# Extend coverage without importing any Rust parser or normalizer.
oracle.CASES = [
    ('definition_beta', [('def', 'main'), ('projects', 'beta')]),
    ('reference_alpha', [('symbol', 'main'), ('projects', 'alpha')]),
]
oracle.main()
