"""Safety contract at the public oracle capture command boundary."""
import json
import os
import platform
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMMAND = ROOT / 'tools/capture_opengrok_baseline.py'
CACHE = Path.home() / 'Library/Caches/rustgrok/oracle'
READY = (platform.system() == 'Darwin' and platform.machine() == 'arm64' and
         all((CACHE / path).exists() for path in (
             'toolchains/OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz',
             'toolchains/jdk-21.0.13+9/Contents/Home/bin/java',
             'toolchains/apache-tomcat-10.1.60.tar.gz',
             'toolchains/apache-tomcat-10.1.60/conf/server.xml',
             'opengrok-1.14.20/opengrok-1.14.20.tar.gz',
             'opengrok-1.14.20/release/opengrok-1.14.20/lib/opengrok.jar',
             'opengrok-1.14.20/release/opengrok-1.14.20/lib/source.war')) and
         Path('/opt/homebrew/bin/ctags').is_file())


class OracleCaptureContract(unittest.TestCase):
    def test_rejects_missing_corpus_before_creating_outputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'capture'
            result = subprocess.run(['python3', str(COMMAND), '--corpus', str(Path(tmp) / 'absent'),
                                     '--output', str(output)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('corpus manifest', result.stderr)
            self.assertFalse(output.exists())


    def test_rejects_ignored_extra_source_even_if_git_status_is_clean(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / 'corpus'
            subprocess.run(['python3', str(ROOT / 'tools/build_public_corpus.py'), '--output', str(corpus)],
                           check=True, capture_output=True)
            repo = corpus / 'source/alpha/component-c'
            (repo / '.git/info').mkdir(exist_ok=True)
            with (repo / '.git/info/exclude').open('a') as exclude:
                exclude.write('private.c\n')
            (repo / 'private.c').write_text('private material')
            self.assertEqual(subprocess.check_output(['git', '-C', str(repo), 'status', '--porcelain'], text=True), '')
            output = Path(tmp) / 'capture'
            result = subprocess.run(['python3', str(COMMAND), '--corpus', str(corpus),
                                     '--output', str(output), '--cache', str(Path(tmp) / 'missing-cache')],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('unexpected source files', result.stderr)
            self.assertFalse(output.exists())


    def test_ambient_git_directory_cannot_replace_the_public_repo(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / 'corpus'
            subprocess.run(['python3', str(ROOT / 'tools/build_public_corpus.py'), '--output', str(corpus)],
                           check=True, capture_output=True)
            environment = os.environ.copy()
            environment['GIT_DIR'] = str(Path(tmp) / 'wrong-repository')
            output = Path(tmp) / 'capture'
            result = subprocess.run(['python3', str(COMMAND), '--corpus', str(corpus),
                                     '--output', str(output), '--cache', str(Path(tmp) / 'missing-cache')],
                                    capture_output=True, text=True, env=environment)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz', result.stderr)
            self.assertFalse(output.exists())


    def test_refuses_to_write_output_inside_the_input_corpus(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / 'corpus'
            subprocess.run(['python3', str(ROOT / 'tools/build_public_corpus.py'), '--output', str(corpus)],
                           check=True, capture_output=True)
            output = corpus / 'source/alpha/new-index'
            result = subprocess.run(['python3', str(COMMAND), '--corpus', str(corpus),
                                     '--output', str(output), '--cache', str(Path(tmp) / 'missing-cache')],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('output must not be inside corpus', result.stderr)
            self.assertFalse(output.exists())


    @unittest.skipUnless(READY, 'verified macOS oracle inputs not installed')
    def test_oracle_ignores_ambient_java_option_injection(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / 'corpus'
            subprocess.run(['python3', str(ROOT / 'tools/build_public_corpus.py'), '--output', str(corpus)],
                           check=True, capture_output=True)
            environment = os.environ.copy()
            for name in ('JAVA_TOOL_OPTIONS', 'JDK_JAVA_OPTIONS', '_JAVA_OPTIONS', 'JAVA_OPTS'):
                environment[name] = '-XX:NotARealJavaFlag'
            result = subprocess.run(['python3', str(COMMAND), '--corpus', str(corpus),
                                     '--output', str(Path(tmp) / 'capture')],
                                    capture_output=True, text=True, env=environment, timeout=120)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)['statuses'], [200] * 5)

    @unittest.skipUnless(READY, 'verified macOS oracle inputs not installed')
    def test_rejects_changed_tomcat_config_before_writing_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp); corpus = base / 'corpus'; fake = base / 'cache'; ft = fake / 'toolchains'
            subprocess.run(['python3', str(ROOT / 'tools/build_public_corpus.py'), '--output', str(corpus)],
                           check=True, capture_output=True)
            ft.mkdir(parents=True)
            original = CACHE / 'toolchains'
            for name in ('OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz',
                         'apache-tomcat-10.1.60.tar.gz', 'jdk-21.0.13+9'):
                (ft / name).symlink_to(original / name)
            shutil.copytree(original / 'apache-tomcat-10.1.60', ft / 'apache-tomcat-10.1.60', symlinks=True)
            server = ft / 'apache-tomcat-10.1.60/conf/server.xml'
            server.write_bytes(server.read_bytes() + b'\nBROKEN')
            (fake / 'opengrok-1.14.20').symlink_to(CACHE / 'opengrok-1.14.20')
            output = base / 'capture'
            result = subprocess.run(['python3', str(COMMAND), '--corpus', str(corpus),
                                     '--output', str(output), '--cache', str(fake)],
                                    capture_output=True, text=True, timeout=120)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Tomcat runtime mismatch', result.stderr)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
