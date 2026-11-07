"""Regression: renewal and rollback must retain the deployment's Compose inputs.

Filesystem operations are real; certificate validation and Docker are mocked.
"""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('tls_maintenance', ROOT / 'scripts/production/tls-maintenance.py')
tls = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tls)


class DeploymentInputs(unittest.TestCase):
    def exercise(self, rollback):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            private = root / '.production'
            current = private / 'tls'
            current.mkdir(parents=True)
            source = private / 'tls-renewal-pending' if rollback else root / 'renewed'
            source.mkdir()
            for name in ['web.crt', 'web.key', 'broker.crt', 'broker.key', 'ca.crt']:
                (current / name).write_text('ca' if name == 'ca.crt' else 'old')
                (source / name).write_text('ca' if name == 'ca.crt' else 'replacement')
            (private / 'deployment.json').write_text(json.dumps({'web_host': 'web.example', 'broker_host': 'broker.example'}))
            env = root / 'production.env'
            overlay = root / 'network.yml'
            env.write_text('HISTORY_RETENTION_ENABLED=false\n')
            overlay.write_text('services: {}\n')
            argv = ['tls-maintenance.py', '--project-name', 'office-production',
                    '--env-file', str(env), '--compose-file', str(overlay)]
            argv += ['--rollback-pending'] if rollback else ['--install-from', str(source)]
            # Emulate the Linux guard without changing pathlib's host implementation.
            platform = SimpleNamespace(name='posix', umask=lambda _: None, replace=os.replace)
            with patch.object(tls, 'ROOT', root), patch.object(tls, 'os', platform), \
                 patch.object(tls, 'check'), patch.object(tls.subprocess, 'run') as run, \
                 patch('sys.argv', argv):
                tls.main()
            commands = [call.args[0] for call in run.call_args_list]
            self.assertTrue(commands)
            for command in commands:
                self.assertEqual(command[command.index('--project-name') + 1], 'office-production')
                self.assertEqual(command[command.index('--env-file') + 1], str(env.resolve()))
                self.assertIn(str(overlay.resolve()), command)
            self.assertEqual(sum('up' in command for command in commands), 1)
            self.assertEqual((current / 'web.crt').read_text(), 'replacement')
            self.assertFalse((private / 'tls-renewal-pending').exists())

    def test_renewal(self):
        self.exercise(False)

    def test_rollback(self):
        self.exercise(True)


if __name__ == '__main__':
    unittest.main()
