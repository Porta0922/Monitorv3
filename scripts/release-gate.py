"""Release only a version whose exact main commit passed all required CI."""
import json
import os
import subprocess
import time
import tomllib


def api(path):
    return json.loads(subprocess.check_output(['gh', 'api', path]))


def main():
    repo = os.environ['GITHUB_REPOSITORY']
    sha = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    with open('agent/Cargo.toml', 'rb') as source:
        version = tomllib.load(source)['package']['version']
    required = {'Publication preflight', 'Storage retention', 'Agent lifecycle checks', 'Stack smoke test'}
    for _ in range(60):
        runs = api(f'repos/{repo}/actions/runs?head_sha={sha}&event=push&per_page=100')['workflow_runs']
        latest = {}
        for run in sorted(runs, key=lambda item: item['id']):
            if run['name'] in required:
                latest[run['name']] = run
        if len(latest) == len(required) and all(run['status'] == 'completed' for run in latest.values()):
            if any(run['conclusion'] != 'success' for run in latest.values()):
                raise SystemExit('Required CI failed; no release')
            break
        time.sleep(15)
    else:
        raise SystemExit('Required CI did not complete; no release')
    releases = api(f'repos/{repo}/releases?per_page=100')
    needed = not any(item['tag_name'] == f'agent-v{version}' for item in releases)
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:
        output.write(f'version={version}\nsha={sha}\nneeded={str(needed).lower()}\n')


if __name__ == '__main__':
    main()
