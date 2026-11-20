import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('release_gate', ROOT / 'scripts/release-gate.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class ReleaseGateTests(unittest.TestCase):
    def run_gate(self, failed=False, existing=False, missing=False):
        names = ['Publication preflight', 'Storage retention', 'Agent lifecycle checks', 'Stack smoke test']
        runs = [{'id': i, 'name': name, 'status': 'completed', 'conclusion': 'success'} for i, name in enumerate(names)]
        if failed:
            runs[-1]['conclusion'] = 'failure'
        if missing:
            runs.pop()
        import tomllib
        version = tomllib.loads((ROOT / 'agent/Cargo.toml').read_text())['package']['version']
        def api(path):
            return {'workflow_runs': runs} if '/actions/' in path else ([{'tag_name': f'agent-v{version}'}] if existing else [])
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'output'
            with patch.dict(os.environ, {'GITHUB_REPOSITORY': 'Porta0922/Monitorv3', 'GITHUB_OUTPUT': str(output)}), patch.object(gate, 'api', api), patch.object(gate.subprocess, 'check_output', return_value='commit'), patch.object(gate.time, 'sleep'):
                gate.main()
            return output.read_text()

    def test_requires_every_check_to_pass(self):
        with self.assertRaises(SystemExit):
            self.run_gate(failed=True)
        with self.assertRaises(SystemExit):
            self.run_gate(missing=True)

    def test_existing_version_is_never_overwritten(self):
        self.assertIn('needed=false', self.run_gate(existing=True))

    def test_success_enables_release(self):
        self.assertIn('needed=true', self.run_gate())


if __name__ == '__main__':
    unittest.main()
