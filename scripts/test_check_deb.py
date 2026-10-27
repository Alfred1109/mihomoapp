"""Dependency inspection regressions; uses temporary packages and never installs them."""
import hashlib
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('check_deb', Path(__file__).with_name('check-deb.py'))
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class DependencyTests(unittest.TestCase):
    def test_unrelated_alternative_cannot_satisfy_abi(self):
        required = [('libwebkit2gtk-4.0-37', None, None)]
        self.assertFalse(CHECK.covers(CHECK.dependencies('libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37'), required))
        self.assertTrue(CHECK.covers(CHECK.dependencies('libwebkit2gtk-4.0-37 (>= 2.40)'), required))

    def test_version_bounds_are_not_weakened(self):
        required = CHECK.dependencies('libc6 (>= 2.34)')[0]
        for declaration in ['libc6', 'libc6 (>= 2.31)', 'libc6 (>= 2.34) | zlib1g']:
            self.assertFalse(CHECK.covers(CHECK.dependencies(declaration), required))
        self.assertTrue(CHECK.covers(CHECK.dependencies('libc6 (>= 2.35)'), required))

    def test_package_is_inspected_without_rewriting(self):
        with tempfile.TemporaryDirectory(prefix='deb-regression-') as temporary:
            root = Path(temporary)
            (root / 'debian').mkdir()
            (root / 'debian/control').write_text('Source: inspection-test\n\nPackage: inspection-test\nArchitecture: any\nDescription: test\n')
            computed = CHECK.run('dpkg-shlibdeps', '-O', '-e/usr/bin/true', cwd=root)
            depends = next(line.split('=', 1)[1] for line in computed.splitlines() if line.startswith('shlibs:Depends='))
            package_root = root / 'package'
            (package_root / 'DEBIAN').mkdir(parents=True)
            (package_root / 'usr/bin').mkdir(parents=True)
            shutil.copyfile('/usr/bin/true', package_root / 'usr/bin/inspection-test')
            (package_root / 'usr/bin/inspection-test').chmod(0o755)
            control = package_root / 'DEBIAN/control'
            def package(dependencies):
                control.write_text('Package: inspection-test\nVersion: 1.0\nArchitecture: amd64\nMaintainer: Test <test@example.invalid>\nDescription: Temporary test\nDepends: ' + dependencies + '\n')
                output = root / 'test.deb'
                CHECK.run('dpkg-deb', '--build', str(package_root), str(output))
                return output
            output = package(depends)
            digest = hashlib.sha256(output.read_bytes()).hexdigest()
            CHECK.inspect(output)
            self.assertEqual(digest, hashlib.sha256(output.read_bytes()).hexdigest())
            output = package('zlib1g')
            digest = hashlib.sha256(output.read_bytes()).hexdigest()
            with self.assertRaisesRegex(RuntimeError, 'does not guarantee'):
                CHECK.inspect(output)
            self.assertEqual(digest, hashlib.sha256(output.read_bytes()).hexdigest())


if __name__ == '__main__':
    unittest.main()
