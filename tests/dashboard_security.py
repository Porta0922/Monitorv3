"""Browser XSS regression with synthetic API responses and built dashboard assets.

Checks escaping with CSP disabled, then checks the actual production CSP string.
Does not contact the pilot or certify production cookies/backend authorization.
Requires Python Playwright/Chromium and dashboard/dist from the current checkout.
"""
import json
import mimetypes
import re
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / 'dashboard/dist'
BASE = 'http://monitor-security.test'
STAMP = datetime.now(timezone.utc).isoformat()
DAY = STAMP[:10]
PAYLOAD = "<img src=x onerror='window.__monitorXss=1'>"
CSP = re.search(r'add_header Content-Security-Policy "([^"]+)"',
                (ROOT / 'dashboard/nginx.production.conf').read_text()).group(1)


def verify(browser, enforce_csp):
    context = browser.new_context(viewport={'width': 1440, 'height': 1000})
    context.add_init_script("window.__monitorXss=0; window.__cspViolations=[]; document.addEventListener('securitypolicyviolation',e=>window.__cspViolations.push(e.effectiveDirective));")
    page = context.new_page()
    errors, unexpected = [], []
    page.on('pageerror', lambda error: errors.append(str(error)))
    device = dict(device_id='security-device', hostname=PAYLOAD, mac_address='', last_seen=STAMP, online=True, created_at=STAMP)

    def handle(route):
        url = urlparse(route.request.url)
        assert url.netloc == 'monitor-security.test', 'Unexpected external browser request'
        path = url.path
        if not path.startswith('/api/'):
            if path == '/x': unexpected.append(path)
            asset = (DIST / path.lstrip('/')).resolve()
            if not asset.is_relative_to(DIST.resolve()): route.fulfill(status=404); return
            if not asset.is_file(): asset = DIST / 'index.html'
            route.fulfill(status=200, content_type=mimetypes.guess_type(asset.name)[0] or 'application/octet-stream',
                          headers={'Content-Security-Policy': CSP} if enforce_csp else {}, body=asset.read_bytes())
            return
        if path == '/api/auth/session': data = dict(username='security-test', role='operator', csrf_token='synthetic', expires_in=3600)
        elif path == '/api/devices': data = {'devices': [device]}
        elif path.endswith('/telemetry'): data = {'resources': None, 'health': None}
        elif path.endswith('/sessions'): data = dict(day=DAY, timezone='UTC', total=0, totals={}, items=[])
        elif path == '/api/history_hourly_programs':
            data = {'success': True, 'groups': [dict(hour=10, app_name=PAYLOAD, window_title=PAYLOAD, total_seconds=120, count=1)]}
        elif path == '/api/usb/security-device': data = {'events': [dict(timestamp=STAMP, device_id='security-device', action='IN', hardware_id='synthetic', device_name=PAYLOAD, serial_number='synthetic')]}
        elif path == '/api/wifi/security-device': data = {'events': [dict(timestamp=STAMP, device_id='security-device', interface_name='WiFi', state='connected', ssid=PAYLOAD, signal_percent=70)]}
        elif path == '/api/live_devices': data = {'success': True, 'devices': []}
        elif path.startswith('/api/resources/'): data = {'metrics': []}
        elif path.startswith('/api/inventory/current/'): data = dict(sources=[], items=[], total=0)
        elif path.startswith('/api/inventory/'): data = {'apps': []}
        else: data = {}
        route.fulfill(status=200, content_type='application/json', body=json.dumps(data))

    page.route('**/*', handle)
    page.goto(f'{BASE}/devices/security-device?date={DAY}')
    expect(page.get_by_role('heading', name=PAYLOAD, exact=True)).to_be_visible()
    page.get_by_role('button', name='Actividad', exact=True).click()
    page.locator('.activity-app summary').first.click()
    expect(page.locator('.window-row').get_by_text(PAYLOAD, exact=True)).to_be_visible()
    expect(page.get_by_title(PAYLOAD, exact=True)).to_be_visible()
    page.get_by_label('Buscar aplicación o título de ventana').fill(PAYLOAD)
    expect(page.locator('.window-row').get_by_text(PAYLOAD, exact=True)).to_be_visible()
    for tab in ['USB', 'WiFi']:
        page.get_by_role('button', name=tab, exact=True).click()
        expect(page.get_by_role('cell', name=PAYLOAD, exact=True)).to_be_visible()
    assert page.evaluate('window.__monitorXss') == 0
    assert page.locator('img[src="x"]').count() == 0
    assert not unexpected, unexpected
    if enforce_csp:
        page.evaluate("const script=document.createElement('script'); script.textContent='window.__monitorXss=2'; document.body.append(script);")
        page.wait_for_function("window.__cspViolations.some(d=>d.startsWith('script-src'))")
        assert page.evaluate('window.__monitorXss') == 0
    assert not errors, errors
    context.close()


if __name__ == '__main__':
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(headless=True)
        for policy in [False, True]: verify(browser, policy)
        browser.close()
    print('PASS: hostname, application, window title, search, USB and WiFi render as text; production CSP blocks inline script')
