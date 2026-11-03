"""Publish from the native agent transport probe and verify through the dashboard."""
from http_auth import login_headers
import json, os, subprocess, sys, time, urllib.request, uuid
from pathlib import Path
base = os.environ.get("SMOKE_BASE_URL", "http://127.0.0.1:15173")
device = str(uuid.uuid4())
env = dict(os.environ, RABBITMQ_URL=os.environ.get("PROBE_RABBITMQ_URL", "amqp://guest:guest@127.0.0.1:15673/%2f"),
           PROBE_DEVICE_ID=device, PROBE_SECONDS="1", PROBE_CACHE=str(Path("target") / (device + ".sqlite")))
output = subprocess.check_output([sys.argv[1]], env=env, text=True, encoding="utf-8", timeout=60)
result = next(json.loads(line) for line in output.splitlines() if '"probe_passed":true' in line)
def request(path, payload=None, auth=None):
    headers = {"Content-Type":"application/json"}
    if auth: headers.update(auth)
    req = urllib.request.Request(base+path, data=None if payload is None else json.dumps(payload).encode(), headers=headers)
    with urllib.request.urlopen(req, timeout=10) as response: return json.load(response)
auth = login_headers(base, os.environ.get("SMOKE_ADMIN_USER", "admin"), os.environ.get("SMOKE_ADMIN_PASSWORD", "admin"))
for _ in range(30):
    samples = request("/api/resources/"+device, auth=auth).get("metrics", [])
    if len(samples) == result["produced"]: break
    time.sleep(1)
assert len(samples) == result["produced"] > 0
assert all(s["cpu_percent"] == 42.5 for s in samples)
print(json.dumps({"native_transport":"passed","device_id":device,"samples":len(samples)}))
