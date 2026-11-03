"""Destructive only to the named disposable monitor-refactor-test stack.
Exercises actual agent publisher/cache, 90s broker outage, process termination,
and end-to-end persisted metrics. No host agent/service is installed.
"""
from http_auth import login_headers
import base64
import json
import os
import subprocess
import time
import urllib.request
import urllib.error
import uuid

PROJECT = "monitor-refactor-test"
BASE = os.environ.get("SMOKE_BASE_URL", "http://127.0.0.1:15173")
IMAGE = os.environ.get("PROBE_IMAGE", "monitor-p0-validation")
PROBE = "monitor-p0-agent-probe"
BROKER = PROJECT + "-rabbitmq-1"
SERVER = PROJECT + "-server-1"

def docker(*args):
    return subprocess.check_output(["docker", *args], text=True, encoding="utf-8").strip()

def wait_until(check, timeout=120):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            value = check()
            if value:
                return value
        except (OSError, ValueError, subprocess.CalledProcessError):
            pass
        time.sleep(2)
    raise AssertionError("Recovery condition timed out")

def request(path, payload=None, auth=None):
    data = None if payload is None else json.dumps(payload).encode()
    headers = {"Content-Type": "application/json"}
    if auth:
        headers.update(auth)
    with urllib.request.urlopen(urllib.request.Request(BASE + path, data=data, headers=headers), timeout=5) as response:
        return json.load(response)

def broker_request(path, payload=None, method=None):
    url = os.environ.get("SMOKE_BROKER_URL", "http://127.0.0.1:15674")
    headers = {"Content-Type":"application/json", "Authorization":"Basic " + base64.b64encode(b"guest:guest").decode()}
    req = urllib.request.Request(url+path, data=None if payload is None else json.dumps(payload).encode(), headers=headers, method=method)
    with urllib.request.urlopen(req, timeout=10) as response:
        data = response.read()
        return json.loads(data) if data else None

def not_ready():
    try:
        request("/api/ready")
    except urllib.error.HTTPError as error:
        return error.code == 503
    return False


def main():
    # Refuse to operate on containers outside the known test Compose project.
    for name in [SERVER, BROKER]:
        assert docker("inspect", "--format", '{{index .Config.Labels "com.docker.compose.project"}}', name) == PROJECT
    auth = login_headers(BASE, os.environ.get("SMOKE_ADMIN_USER", "admin"), os.environ.get("SMOKE_ADMIN_PASSWORD", "admin"))
    device = str(uuid.uuid4())
    docker("run", "-d", "--name", PROBE, "--network", PROJECT + "_activity-monitor",
           "-e", "RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672/%2f",
           "-e", "PROBE_SECONDS=135", "-e", "PROBE_DEVICE_ID=" + device,
           IMAGE, "/app/target/release/agent-transport-probe")
    stopped = False
    try:
        wait_until(lambda: request("/api/resources/" + device, auth=auth).get("metrics"))
        before = int(docker("inspect", "--format", "{{.RestartCount}}", SERVER))
        # Docker kill is treated as an administrative stop. Terminate the actual
        # child process under Docker's init to simulate an unexpected crash.
        assert docker("inspect", "--format", "{{.HostConfig.Init}}", SERVER) == "true"
        docker("exec", SERVER, "sh", "-c",
               'set -- $(cat /proc/1/task/1/children); [ "$#" -eq 1 ] && '
               '[ "$(readlink /proc/$1/exe)" = "/usr/local/bin/activity-monitor-server" ] && kill -KILL "$1"')
        wait_until(lambda: int(docker("inspect", "--format", "{{.RestartCount}}", SERVER)) > before)
        wait_until(lambda: request("/api/ready").get("status") == "ready")
        print("Server automatically recovered after SIGKILL", flush=True)
        docker("stop", BROKER)
        stopped = True
        wait_until(not_ready, 30)
        start = time.monotonic()
        while time.monotonic() - start < 90:
            time.sleep(2)
        assert any(json.loads(line).get("pending", 0) > 0 for line in docker("logs", PROBE).splitlines() if line.startswith("{"))
        docker("start", BROKER)
        stopped = False
        print("Broker restored after 90 seconds; waiting for cache replay", flush=True)
        wait_until(lambda: docker("inspect", "--format", "{{.State.Running}}", PROBE) == "false", 210)
        assert docker("inspect", "--format", "{{.State.ExitCode}}", PROBE) == "0", docker("logs", PROBE)
        result = next(json.loads(line) for line in docker("logs", PROBE).splitlines() if '"probe_passed":true' in line)
        assert result["max_pending"] > 0
        metrics = wait_until(lambda: (m if len(m := request("/api/resources/" + device, auth=auth).get("metrics", [])) == result["produced"] else None))
        assert all(m["cpu_percent"] == 42.5 for m in metrics)
        # A persistent message must survive container recreation, not only restart.
        queue = "p0_persistence_" + uuid.uuid4().hex
        broker_request("/api/queues/%2F/" + queue, {"durable":True,"auto_delete":False,"arguments":{}}, "PUT")
        assert broker_request("/api/exchanges/%2F/amq.default/publish", {
            "properties":{"delivery_mode":2},"routing_key":queue,
            "payload":"persisted-across-recreate","payload_encoding":"string"})["routed"]
        hostname = docker("inspect", "--format", "{{.Config.Hostname}}", BROKER)
        docker("compose", "-p", PROJECT, "-f", "docker-compose.yml", "-f", "tests/compose.smoke.yml",
               "up", "-d", "--no-deps", "--force-recreate", "rabbitmq")
        wait_until(lambda: docker("inspect", "--format", "{{.State.Health.Status}}", BROKER) == "healthy", 150)
        wait_until(lambda: request("/api/ready").get("status") == "ready", 150)
        assert hostname == docker("inspect", "--format", "{{.Config.Hostname}}", BROKER)
        messages = broker_request("/api/queues/%2F/" + queue + "/get", {
            "count":1,"ackmode":"ack_requeue_false","encoding":"auto","truncate":1000})
        assert len(messages) == 1 and messages[0]["payload"] == "persisted-across-recreate"
        broker_request("/api/queues/%2F/" + queue, method="DELETE")
        result.update({"broker_outage_seconds":90,"server_automatic_restart":True,
                       "persisted_samples":len(metrics),"broker_message_survived_recreate":True})
        print(json.dumps(result), flush=True)
    finally:
        if stopped:
            docker("start", BROKER)
        docker("rm", "-f", PROBE)

if __name__ == "__main__":
    main()
