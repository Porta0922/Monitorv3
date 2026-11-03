"""Explicit session headers for integration checks; never authenticate anonymous probes implicitly."""
import http.cookies
import json
import urllib.request


def login_headers(base, username="admin", password="admin", otp=""):
    request = urllib.request.Request(
        base.rstrip("/") + "/api/auth/login",
        data=json.dumps({"username": username, "password": password, "otp": otp}).encode(),
        headers={"Content-Type": "application/json", "X-Monitor-Client": "dashboard"},
    )
    with urllib.request.urlopen(request, timeout=15) as response:
        result = json.load(response)
        cookies = http.cookies.SimpleCookie()
        for value in response.headers.get_all("Set-Cookie", []):
            cookies.load(value)
    name = next((key for key in ("__Host-monitor_session", "monitor_session") if key in cookies), None)
    if not name or not result.get("csrf_token"):
        raise AssertionError("Login did not issue a session cookie and CSRF token")
    return {"Cookie": name + "=" + cookies[name].value, "X-CSRF-Token": result["csrf_token"]}
