#!/usr/bin/env python3
"""Measure real, private editor paths in an isolated source copy (no public test API).

Usage: python3 scripts/editor_alloc_probe.py [--check]
Release-mode CPU/egui frames, not GPU or window latency. Cargo dependencies/build outputs are
shared with this checkout; the temporary source tree and its counting allocator are discarded.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='enforce measured scaling guards')
    parser.add_argument('--restore', choices=('table', 'sort', 'clipboard'),
                        help='sabotage one optimization, only in the temporary source copy')
    parser.add_argument('--baseline-ref', help='Git ref to restore that file from (required with --restore)')
    args = parser.parse_args()
    if bool(args.restore) != bool(args.baseline_ref):
        parser.error('--restore and --baseline-ref must be supplied together')
    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix='skeleton-editor-alloc-') as directory:
        scratch = Path(directory)
        for name in ('src', 'engine_reflect_derive'):
            shutil.copytree(root / name, scratch / name,
                            ignore=shutil.ignore_patterns('target', '.git'))
        for name in ('Cargo.toml', 'Cargo.lock', 'README.md'):
            shutil.copy2(root / name, scratch / name)
        if args.restore:
            relative = {
                'table': 'src/app/editor/ui/data_table_panel.rs',
                'sort': 'src/app/editor/ui/docked/entity_kind.rs',
                'clipboard': 'src/app/editor/ui/docked/inspector_tab.rs',
            }[args.restore]
            (scratch / relative).write_bytes(subprocess.check_output(
                ['git', 'show', f'{args.baseline_ref}:{relative}'], cwd=root))
        # Compile-time font/fixture paths and example declarations still resolve; they are read only.
        for name in ('assets', 'examples', 'tests', '.cargo'):
            (scratch / name).symlink_to(root / name, target_is_directory=True)
        ui = scratch / 'src/app/editor/ui/mod.rs'
        probe = root / 'scripts/probes/editor_alloc.rs'
        with ui.open('a') as stream:
            stream.write('\n#[cfg(test)]\n#[path = ' + json.dumps(str(probe))
                         + ']\nmod alloc_probe;\n')
        # Only visibility changes, solely in the disposable copy, to measure the actual sort.
        docked = scratch / 'src/app/editor/ui/docked/mod.rs'
        docked.write_text(docked.read_text().replace('mod entity_kind;', 'pub(super) mod entity_kind;'))
        kind = scratch / 'src/app/editor/ui/docked/entity_kind.rs'
        kind.write_text(kind.read_text().replace('pub(super) fn sorted_entity_list(',
                                                'pub(in crate::app) fn sorted_entity_list('))
        env = os.environ.copy()
        env['CARGO_TARGET_DIR'] = str(root / 'target')
        env['SKELETON_MUTE'] = '1'
        env['EDITOR_ALLOC_CHECK'] = '1' if args.check else '0'
        process = subprocess.Popen(['cargo', 'test', '--manifest-path', str(scratch / 'Cargo.toml'),
                                 '--release', '--lib', 'app::editor::ui::alloc_probe::editor_allocations', '--',
                                 '--exact', '--nocapture', '--test-threads=1'], env=env,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        measured = passed = False
        for line in process.stdout:
            print(line, end="", flush=True)
            measured |= 'EDITOR_ALLOC table/10000 ' in line
            passed |= 'test result: ok. 1 passed;' in line
        status = process.wait()
        if status == 0 and not (measured and passed):
            print('FAIL: the named allocation test did not run to completion', file=sys.stderr)
            return 1
        return status


if __name__ == '__main__':
    raise SystemExit(main())
