"""Real Linux agent process in an isolated container; no host collectors or services.
Uses Docker on-failure recovery, not systemd. Keeps local evidence in target/.
"""
import argparse, json, pathlib, sqlite3, subprocess, time, uuid


def docker(*args):
    result = subprocess.run(["docker", *args], capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=45)
    if result.returncode:
        raise RuntimeError(f"docker {args[0]} failed: {result.stderr}")
    return (result.stdout + (result.stderr if args[0] == "logs" else "")).strip()


def wait_for(check, timeout=40):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        value = check()
        if value:
            return value
        time.sleep(0.25)
    raise AssertionError("Timed out waiting for agent lifecycle state")


def run(image):
    name = "monitor-agent-runtime-" + uuid.uuid4().hex[:10]
    root = pathlib.Path("target", name).resolve()
    root.mkdir(parents=True)
    policy = dict(window_activity_interval_secs=2, heartbeat_interval_secs=60,
        usb_detector_interval_secs=60, usb_copy_interval_secs=60, wifi_interval_secs=120,
        running_apps_interval_secs=60, inventory_interval_days=30, heatmap_interval_secs=3600,
        resource_logger_interval_secs=60, osquery_scheduler_seconds=0, idle_threshold_seconds=300,
        activity_heartbeat_seconds=30, enabled_monitors=[], disabled_monitors=[], monitoring_paused=True)
    (root / "effective_policy.json").write_text(json.dumps(policy), encoding="utf-8")
    (root / "test.env").write_text("", encoding="utf-8")
    created = False
    volume = name + "-data"
    snapshot_index = 0
    def inspect():
        return json.loads(docker("inspect", name))[0]
    def ready():
        info = inspect()
        if not info["State"]["Running"]:
            return False
        return "Agent started successfully" in docker("logs", "--since", info["State"]["StartedAt"], name)
    def snapshot():
        nonlocal snapshot_index
        snapshot_index += 1
        copied = root / f"snapshot-{snapshot_index}"
        copied.mkdir()
        docker("cp", name + ":/test-data/.", str(copied))
        identity = json.loads((copied / "device_id.json").read_text(encoding="utf-8"))
        with sqlite3.connect(copied / "agent_service_cache.db") as db:
            assert db.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
            ids = {row[0] for row in db.execute("SELECT id FROM cache_events WHERE synced=0")}
        assert ids, "Health event should remain queued while broker is unavailable"
        return identity, ids
    try:
        docker("create", "--init", "--name", name, "--restart", "on-failure:3", "--network", "none",
            "--mount", f"type=volume,source={volume},target=/test-data",
            "-e", "AGENT_MODE=service", "-e", "AGENT_DATA_DIR=/test-data",
            "-e", "AGENT_LOG_DIR=/test-data/logs", "-e", "AGENT_CONFIG_FILE=/test-data/test.env",
            "-e", "AGENT_SERVER_URL=http://127.0.0.1:9", "-e", "RABBITMQ_URL=amqp://guest:guest@127.0.0.1:9/%2f",
            "-e", "AGENT_AUTH_TOKEN=isolated-runtime-test", "-e", "AGENT_SESSION_RELAY_ENABLED=false",
            "-e", "WEB_BIND=127.0.0.1:9876", image, "/usr/local/bin/activity-monitor-agent")
        created = True
        for filename in ["effective_policy.json", "test.env"]:
            docker("cp", str(root / filename), name + ":/test-data/" + filename)
        docker("start", name)
        wait_for(ready)
        docker("stop", "--time", "10", name)
        first = inspect()
        assert first["State"]["ExitCode"] == 0 and first["RestartCount"] == 0, first["State"]
        identity, pending = snapshot()
        time.sleep(1)
        assert not inspect()["State"]["Running"], "Administrative stop must remain stopped"
        docker("start", name)
        wait_for(ready)
        time.sleep(11)
        children = docker("exec", name, "cat", "/proc/1/task/1/children").split()
        assert len(children) == 1 and children[0].isdigit(), "Expected only the agent under container init"
        docker("exec", name, "sh", "-c", 'kill -KILL "$1"', "sh", children[0])
        wait_for(lambda: inspect()["RestartCount"] >= 1)
        wait_for(ready)
        restart_count = inspect()["RestartCount"]
        docker("stop", "--time", "10", name)
        assert inspect()["State"]["ExitCode"] == 0
        new_identity, new_pending = snapshot()
        assert identity == new_identity, "Restart changed device identity"
        assert pending <= new_pending, "Restart lost unacknowledged events"
        result = dict(status="passed", image=image, graceful_exit=0, crash_restarts=restart_count,
            identity_preserved=True, pending_before=len(pending), pending_after=len(new_pending),
            scope="Linux service mode under Docker, not installed systemd/SCM")
        (root / "result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        print(json.dumps(result))
    finally:
        if created:
            (root / "container.log").write_text(docker("logs", name), encoding="utf-8")
            docker("rm", "-f", name)
            docker("volume", "rm", volume)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--image", default="monitor-agent-debian-validation")
    run(parser.parse_args().image)
