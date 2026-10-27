#!/usr/bin/env python3
"""Read-only inspection of generated Debian metadata and native ELF requirements."""
import argparse
import re
import subprocess
import tempfile
from pathlib import Path

DEPENDENCY = re.compile(r"^([a-z0-9][a-z0-9+.-]*)(?::[a-z0-9-]+)?(?:\s*\((<<|<=|=|>=|>>)\s*([^()]+)\))?$")


def run(*args, cwd=None):
    result = subprocess.run(args, cwd=cwd, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(f"{' '.join(map(str, args))} failed:\n{result.stderr.strip()}")
    return result.stdout.strip()


def dependencies(value):
    groups = []
    for group in value.split(','):
        if not group.strip():
            continue
        choices = []
        for choice in group.split('|'):
            match = DEPENDENCY.fullmatch(choice.strip())
            if not match:
                raise RuntimeError(f"Cannot safely interpret dependency: {choice.strip()}")
            choices.append(match.groups())
        groups.append(choices)
    return groups


def implies(actual, required):
    name, operator, version = actual
    req_name, req_operator, req_version = required
    if name != req_name:
        return False
    if req_operator is None:
        return True
    if operator is None:
        return False
    if req_operator in ('>=', '>>') and operator in ('>=', '>>', '='):
        relation = 'gt' if req_operator == '>>' and operator != '>>' else 'ge'
    elif req_operator == '=' and operator == '=':
        relation = 'eq'
    else:
        return False
    return subprocess.run(['dpkg', '--compare-versions', version, relation, req_version]).returncode == 0


def covers(actual_groups, required_group):
    # An unrelated alternative must not bypass a real ABI requirement.
    return any(all(any(implies(choice, required) for required in required_group)
                   for choice in group) for group in actual_groups)


def inspect(package):
    metadata = run('dpkg-deb', '-f', str(package), 'Depends')
    actual = dependencies(metadata)
    if not actual:
        raise RuntimeError('Generated package has no Depends field')
    with tempfile.TemporaryDirectory(prefix='mihomo-deb-check-') as temporary:
        work = Path(temporary)
        extracted = work / 'root'
        run('dpkg-deb', '-x', str(package), str(extracted))
        elfs = []
        needed = set()
        for file in extracted.rglob('*'):
            if file.is_symlink() or not file.is_file():
                continue
            with file.open('rb') as stream:
                if stream.read(4) != b'\x7fELF':
                    continue
            dynamic = run('readelf', '-d', str(file))
            libraries = re.findall(r'\(NEEDED\).*?\[([^\]]+)\]', dynamic)
            if libraries:
                elfs.append(file)
                needed.update(libraries)
        if not elfs:
            raise RuntimeError('No dynamically linked ELF application found')
        known = {
            'libwebkit2gtk-4.0.so.37': 'libwebkit2gtk-4.0-37',
            'libwebkit2gtk-4.1.so.0': 'libwebkit2gtk-4.1-0',
            'libjavascriptcoregtk-4.0.so.18': 'libjavascriptcoregtk-4.0-18',
            'libjavascriptcoregtk-4.1.so.0': 'libjavascriptcoregtk-4.1-0',
            'libsoup-2.4.so.1': 'libsoup2.4-1',
            'libsoup-3.0.so.0': 'libsoup-3.0-0',
        }
        for soname, runtime in known.items():
            if soname in needed and not covers(actual, [(runtime, None, None)]):
                raise RuntimeError(f'ELF requires {soname}, but Depends does not unconditionally require {runtime}')
        (work / 'debian').mkdir()
        (work / 'debian/control').write_text('Source: mihomoapp-inspection\nSection: utils\nPriority: optional\nMaintainer: Local verification <nobody@example.invalid>\n\nPackage: mihomoapp-inspection\nArchitecture: any\nDescription: Temporary read-only dependency inspection\n')
        computed = run('dpkg-shlibdeps', '-O', *[f'-e{file}' for file in elfs], cwd=work)
        required = next((line.split('=', 1)[1] for line in computed.splitlines()
                         if line.startswith('shlibs:Depends=')), None)
        if required is None:
            raise RuntimeError('dpkg-shlibdeps did not report its computed dependencies')
        missing = [group for group in dependencies(required) if not covers(actual, group)]
        if missing:
            raise RuntimeError(f'Package metadata does not guarantee computed runtime requirements: {missing}\nGenerated Depends: {metadata}\nComputed Depends: {required}')
        print(f'{package.name}: metadata and ELF dependency check passed')
        print(f'  NEEDED: {", ".join(sorted(needed))}')
        print(f'  Depends: {metadata}')
        print(f'  Computed minimum requirements: {required}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', nargs='?', type=Path, default=Path(__file__).resolve().parent.parent / 'tauri-app/backend/target/release/bundle/deb')
    args = parser.parse_args()
    packages = [args.path] if args.path.is_file() else sorted(args.path.glob('*.deb'))
    if not packages:
        parser.error(f'No .deb packages found under {args.path}')
    for package in packages:
        inspect(package.resolve())


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError) as error:
        raise SystemExit(f'Debian verification failed: {error}')
